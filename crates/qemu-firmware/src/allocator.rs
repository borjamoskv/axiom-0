use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicUsize, Ordering};

pub const HEAP_BYTES: usize = 4 * 1024 * 1024;

#[repr(C, align(4096))]
pub struct BumpAllocator {
    storage: UnsafeCell<[u8; HEAP_BYTES]>,
    next: AtomicUsize,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        Self {
            storage: UnsafeCell::new([0; HEAP_BYTES]),
            next: AtomicUsize::new(0),
        }
    }

    pub fn used(&self) -> usize {
        self.next.load(Ordering::Relaxed)
    }
}

// SAFETY: storage is accessed only through raw pointers to disjoint ranges
// reserved by CAS. No shared/mutable references to the backing array are made;
// next never decreases and deallocation never reuses a live allocation.
unsafe impl Sync for BumpAllocator {}

// SAFETY: every successful reservation is aligned to Layout::align, lies wholly
// within the static backing allocation, and is disjoint from all earlier ones.
// Checked arithmetic and exhaustion return null without moving next. Methods do
// not allocate, panic or unwind. The storage outlives every returned pointer.
unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let base = self.storage.get().cast::<u8>();
        let base_address = base as usize;
        let mut current = self.next.load(Ordering::Relaxed);
        loop {
            let Some(address) = base_address
                .checked_add(current)
                .and_then(|address| address.checked_add(layout.align() - 1))
            else {
                return null_mut();
            };
            let aligned_address = address & !(layout.align() - 1);
            let Some(offset) = aligned_address.checked_sub(base_address) else {
                return null_mut();
            };
            let Some(end) = offset
                .checked_add(layout.size())
                .filter(|end| *end <= HEAP_BYTES)
            else {
                return null_mut();
            };
            match self.next.compare_exchange_weak(
                current,
                end,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    // SAFETY: the successful CAS reserved [offset, end) within
                    // storage, with the requested alignment and no overlapping allocation.
                    return unsafe { base.add(offset) };
                }
                Err(actual) => current = actual,
            }
        }
    }

    unsafe fn dealloc(&self, _pointer: *mut u8, _layout: Layout) {
        // One boot has a bounded allocation budget. Deliberately do not recycle.
    }
}

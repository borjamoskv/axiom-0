use alloc::boxed::Box;
use core::arch::asm;
use core::fmt::Write;
use core::hint::spin_loop;
use core::ptr;
use core::sync::atomic::{AtomicPtr, AtomicU8, Ordering};
use micro_axiom_0::seqlock::{ReadError, SeqlockCell, Snapshot};

const LANES: usize = 128;
const WRITES: usize = 32_768;
const CHECKPOINT: u8 = 1;
const RUN: u8 = 2;
const DONE: u8 = 3;
const WRITER_FAILED: u8 = 4;

type Cell = SeqlockCell<u64, LANES>;
static CELL: AtomicPtr<Cell> = AtomicPtr::new(ptr::null_mut());
static PHASE: AtomicU8 = AtomicU8::new(0);

fn payload(generation: usize) -> [u64; LANES] {
    core::array::from_fn(|lane| {
        generation as u64 ^ (lane as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
    })
}

fn start_secondary() -> i64 {
    // SAFETY: boot.S defines an aligned EL1 entry at a VA equal to its PA.
    unsafe extern "C" {
        fn secondary_start();
    }
    let status: i64;
    // SAFETY: QEMU virt supplies PSCI over HVC. The one-time CPU_ON call
    // supplies CPU1's MPIDR and the physical entry of its dedicated stack
    // bootstrap. The shared allocation was published before this call.
    unsafe {
        asm!(
            "dsb ishst",
            "hvc #0",
            inout("x0") 0xc400_0003u64 => status,
            in("x1") 1u64,
            in("x2") secondary_start as *const (),
            in("x3") 0u64,
            clobber_abi("C"),
            options(nostack),
        );
    }
    status
}

// SAFETY: the secondary assembly entry provides an aligned private stack,
// initialized BSS and the same Normal Inner Shareable RAM map as CPU0.
#[unsafe(no_mangle)]
pub extern "C" fn firmware_secondary() -> ! {
    let cell = CELL.load(Ordering::Acquire);
    if !cell.is_null() {
        // SAFETY: CPU0 publishes a fully initialized, permanently live Box.
        // The pointer preserves its provenance and both CPUs use shared access.
        let cell = unsafe { &*cell };
        if cell.try_write(payload(1)) == Ok(2) {
            PHASE.store(CHECKPOINT, Ordering::Release);
            while PHASE.load(Ordering::Acquire) != RUN {
                spin_loop();
            }
            let completed = (2..=WRITES).all(|n| cell.try_write(payload(n)) == Ok(2 * n));
            PHASE.store(
                if completed { DONE } else { WRITER_FAILED },
                Ordering::Release,
            );
        } else {
            PHASE.store(WRITER_FAILED, Ordering::Release);
        }
    } else {
        PHASE.store(WRITER_FAILED, Ordering::Release);
    }
    loop {
        spin_loop();
    }
}

fn verify(snapshot: &Snapshot<u64, LANES>, previous: usize) -> crate::Check {
    crate::require(
        snapshot.attempts == 1
            && snapshot.version & 1 == 0
            && (previous..=2 * WRITES).contains(&snapshot.version)
            && snapshot.value == payload(snapshot.version / 2),
        "incoherent, out-of-range or regressing snapshot",
    )
}

pub fn check() -> crate::Check {
    // CPU1 may still be active when a read fails and CPU0 exits the VM.
    let cell: &'static Cell = Box::leak(Box::new(Cell::new(payload(0))));
    CELL.store(ptr::from_ref(cell).cast_mut(), Ordering::Release);
    let status = start_secondary();
    if status != 0 {
        let _ = writeln!(crate::platform::Console, "PSCI CPU_ON status: {status}");
        return Err("PSCI CPU_ON failed");
    }

    // CPU1 holds generation 1 until CPU0 validates it. The host watchdog
    // bounds the run if CPU1 fails before reaching this handshake.
    loop {
        match PHASE.load(Ordering::Acquire) {
            CHECKPOINT => break,
            WRITER_FAILED => return Err("secondary writer failed before checkpoint"),
            _ => spin_loop(),
        }
    }
    let checkpoint = cell.try_read(1).map_err(|_| "checkpoint read failed")?;
    crate::require(checkpoint.version == 2, "wrong checkpoint version")?;
    verify(&checkpoint, 2)?;
    PHASE.store(RUN, Ordering::Release);

    let mut coherent = 1usize;
    let mut exhausted = 0usize;
    let mut previous = checkpoint.version;
    loop {
        match PHASE.load(Ordering::Acquire) {
            DONE => break,
            WRITER_FAILED => return Err("secondary writer failed during publication"),
            _ => {}
        }
        match cell.try_read(1) {
            Ok(snapshot) => {
                verify(&snapshot, previous)?;
                previous = snapshot.version;
                coherent += 1;
            }
            Err(ReadError::RetryBudgetExhausted { attempts: 1 }) => exhausted += 1,
            Err(_) => return Err("unexpected read error"),
        }
    }
    let final_snapshot = cell.try_read(1).map_err(|_| "final read failed")?;
    verify(&final_snapshot, previous)?;
    crate::require(final_snapshot.version == 2 * WRITES, "wrong final version")?;
    coherent += 1;
    let _ = writeln!(
        crate::platform::Console,
        "SMP SNAPSHOTS: writes={WRITES} coherent={coherent} exhausted={exhausted} final_version={}",
        final_snapshot.version
    );
    crate::require(exhausted > 0, "no contention observed")
}

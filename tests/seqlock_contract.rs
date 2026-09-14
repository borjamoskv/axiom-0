#![cfg(target_has_atomic = "ptr")]

use std::thread;

use micro_axiom_0::seqlock::{AtomicValue, ReadError, SeqlockCell, Snapshot, WriteError};

fn assert_send_sync<T: Send + Sync>() {}

fn assert_copy<T: Copy>() {}

fn assert_error<T: std::error::Error + Send + Sync + 'static>() {}

fn assert_atomic_contract<T: AtomicValue>() {
    assert_send_sync::<T::Atomic>();
    assert_send_sync::<SeqlockCell<T, 0>>();
    assert_send_sync::<SeqlockCell<T, 3>>();
    assert_send_sync::<Snapshot<T, 3>>();
    assert_copy::<Snapshot<T, 3>>();
}

#[test]
fn supported_scalars_preserve_the_public_thread_safety_contract() {
    assert_atomic_contract::<usize>();
    assert_atomic_contract::<isize>();
    #[cfg(target_has_atomic = "8")]
    {
        assert_atomic_contract::<bool>();
        assert_atomic_contract::<u8>();
        assert_atomic_contract::<i8>();
    }
    #[cfg(target_has_atomic = "16")]
    {
        assert_atomic_contract::<u16>();
        assert_atomic_contract::<i16>();
    }
    #[cfg(target_has_atomic = "32")]
    {
        assert_atomic_contract::<u32>();
        assert_atomic_contract::<i32>();
    }
    #[cfg(target_has_atomic = "64")]
    {
        assert_atomic_contract::<u64>();
        assert_atomic_contract::<i64>();
    }
    assert_error::<ReadError>();
    assert_error::<WriteError>();
}

#[test]
fn owned_snapshots_outlive_the_cell_and_cross_thread_boundaries() {
    let snapshot = {
        let cell = SeqlockCell::new([13_usize, 21, 34]);
        let snapshot = cell.try_read(1).unwrap();
        cell.try_write([55, 89, 144]).unwrap();
        snapshot
    };

    let transported = thread::spawn(move || snapshot).join().unwrap();
    assert_eq!(transported, snapshot);
    assert_eq!(transported.value, [13, 21, 34]);
    assert_eq!(transported.version, 0);
    assert_eq!(transported.attempts, 1);
}

//! Bounded attempts at coherent snapshots of native atomic words.
//!
//! `N` is the number of words, not a staleness bound. Each read accepts an
//! explicit retry budget. A successful snapshot may be old, but all its words
//! belong to its reported version. There is no wall-clock latency guarantee.
//!
//! Every payload access is atomic, with Release stores and Acquire loads. A
//! volatile read of an ordinary `T` would not make concurrent writes sound.
//! The sealed [`AtomicValue`] trait therefore permits only native atomic scalar
//! types. No user code, allocation, conversion, or destructor runs while the
//! writer owns the odd version. Sequence numbers never wrap.
//!
//! ```
//! use micro_axiom_0::seqlock::SeqlockCell;
//!
//! let cell = SeqlockCell::<usize, 2>::new([3, 6]);
//! assert_eq!(cell.try_write([4, 8])?, 2);
//! let snapshot = cell.try_read(4)?;
//! assert_eq!(snapshot.value, [4, 8]);
//! assert_eq!(snapshot.version, 2);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Arbitrary objects cannot be read as atomic words:
//!
//! ```compile_fail
//! use micro_axiom_0::seqlock::SeqlockCell;
//! let _ = SeqlockCell::<String, 1>::new([String::from("not atomic")]);
//! ```

use std::{
    fmt,
    sync::atomic::{self, AtomicUsize, Ordering},
};

mod sealed {
    pub trait Sealed {}
}

/// A scalar backed by a native atomic of the same width. Implementations are
/// sealed to bool and signed/unsigned integers up to 64 bits, when supported
/// by the target. This excludes pointers, arbitrary structs, floats and u128.
pub trait AtomicValue: sealed::Sealed + Copy + Send + Sync {
    #[doc(hidden)]
    type Atomic: Send + Sync;
    #[doc(hidden)]
    fn atomic_new(value: Self) -> Self::Atomic;
    #[doc(hidden)]
    fn load_acquire(storage: &Self::Atomic) -> Self;
    #[doc(hidden)]
    fn store_release(storage: &Self::Atomic, value: Self);
}

macro_rules! atomic_value {
    ($value:ty, $atomic:ty) => {
        impl sealed::Sealed for $value {}
        impl AtomicValue for $value {
            type Atomic = $atomic;
            fn atomic_new(value: Self) -> Self::Atomic {
                <$atomic>::new(value)
            }
            fn load_acquire(storage: &Self::Atomic) -> Self {
                storage.load(Ordering::Acquire)
            }
            fn store_release(storage: &Self::Atomic, value: Self) {
                storage.store(value, Ordering::Release);
            }
        }
    };
}

#[cfg(target_has_atomic = "8")]
atomic_value!(bool, atomic::AtomicBool);
#[cfg(target_has_atomic = "8")]
atomic_value!(u8, atomic::AtomicU8);
#[cfg(target_has_atomic = "8")]
atomic_value!(i8, atomic::AtomicI8);
#[cfg(target_has_atomic = "16")]
atomic_value!(u16, atomic::AtomicU16);
#[cfg(target_has_atomic = "16")]
atomic_value!(i16, atomic::AtomicI16);
#[cfg(target_has_atomic = "32")]
atomic_value!(u32, atomic::AtomicU32);
#[cfg(target_has_atomic = "32")]
atomic_value!(i32, atomic::AtomicI32);
#[cfg(target_has_atomic = "64")]
atomic_value!(u64, atomic::AtomicU64);
#[cfg(target_has_atomic = "64")]
atomic_value!(i64, atomic::AtomicI64);
atomic_value!(usize, atomic::AtomicUsize);
atomic_value!(isize, atomic::AtomicIsize);

/// A coherent copy of one complete published version, initially version zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot<T, const N: usize> {
    pub value: [T; N],
    pub version: usize,
    pub attempts: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadError {
    /// No attempt completed with the same even version at both ends.
    /// Zero attempts is valid and returns this error without loading any atomics.
    RetryBudgetExhausted { attempts: usize },
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RetryBudgetExhausted { attempts } => write!(
                f,
                "snapshot retry budget exhausted after {attempts} attempts"
            ),
        }
    }
}

impl std::error::Error for ReadError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteError {
    /// This attempt did not acquire the writer slot. It makes no payload writes.
    Contended,
    /// The next even version would overflow. The last snapshot remains readable.
    VersionExhausted,
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Contended => "snapshot writer slot was not acquired",
            Self::VersionExhausted => "snapshot version space exhausted; counter cannot wrap",
        })
    }
}

impl std::error::Error for WriteError {}

/// A fixed-width snapshot cell with multiple readers and serialized writers.
///
/// The payload is `[T; N]`, each element stored in its own native atomic. All
/// writes replace the entire payload; this is not a read-modify-write API.
/// Fields are private, so callers cannot bypass versioning or downgrade orders.
pub struct SeqlockCell<T: AtomicValue, const N: usize> {
    sequence: AtomicUsize,
    words: [T::Atomic; N],
}

impl<T: AtomicValue, const N: usize> SeqlockCell<T, N> {
    pub fn new(value: [T; N]) -> Self {
        Self {
            sequence: AtomicUsize::new(0),
            words: std::array::from_fn(|i| T::atomic_new(value[i])),
        }
    }

    /// At most `attempts * (N + 2)` atomic loads, with no allocation or locking.
    /// The bound is on program operations, not hardware retries or elapsed time.
    /// Progress is not guaranteed under contention or a paused writer.
    pub fn try_read(&self, attempts: usize) -> Result<Snapshot<T, N>, ReadError> {
        for attempt in 0..attempts {
            let before = self.sequence.load(Ordering::Acquire);
            if before & 1 != 0 {
                continue;
            }
            let value = std::array::from_fn(|i| T::load_acquire(&self.words[i]));
            let after = self.sequence.load(Ordering::Acquire);
            if before == after {
                return Ok(Snapshot {
                    value,
                    version: before,
                    attempts: attempt + 1,
                });
            }
        }
        Err(ReadError::RetryBudgetExhausted { attempts })
    }

    /// One load and at most one strong CAS, N payload stores and one commit.
    /// Failure does not change the payload. Success returns its even version.
    /// No retry loop or user callback runs inside the writer section.
    pub fn try_write(&self, value: [T; N]) -> Result<usize, WriteError> {
        let before = self.sequence.load(Ordering::Acquire);
        if before & 1 != 0 {
            return Err(WriteError::Contended);
        }
        // Checked BEFORE acquisition: exhaustion cannot strand an odd counter.
        let next = before.checked_add(2).ok_or(WriteError::VersionExhausted)?;
        self.sequence
            .compare_exchange(before, before + 1, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| WriteError::Contended)?;
        for (word, value) in self.words.iter().zip(value) {
            // Release/Acquire on each payload word is essential to detect a
            // reader that observes this writer while starting from an old seq.
            T::store_release(word, value);
        }
        self.sequence.store(next, Ordering::Release);
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odd_version_exhausts_exactly_the_budget_without_exposing_partial_data() {
        let cell = SeqlockCell::<usize, 2>::new([10, 20]);
        // Stop a simulated writer between its two stores.
        cell.sequence.store(1, Ordering::Release);
        cell.words[0].store(11, Ordering::Release);
        assert_eq!(
            cell.try_read(7),
            Err(ReadError::RetryBudgetExhausted { attempts: 7 })
        );
        assert_eq!(cell.try_write([99, 99]), Err(WriteError::Contended));
        assert_eq!(cell.words[0].load(Ordering::Acquire), 11);
        assert_eq!(cell.words[1].load(Ordering::Acquire), 20);
        cell.words[1].store(22, Ordering::Release);
        cell.sequence.store(2, Ordering::Release);
        assert_eq!(cell.try_read(1).unwrap().value, [11, 22]);
    }

    #[test]
    fn the_last_even_version_remains_readable_and_cannot_wrap() {
        let cell = SeqlockCell::<usize, 2>::new([1, 2]);
        // Advance an otherwise quiescent cell to its penultimate even version.
        cell.sequence.store(usize::MAX - 3, Ordering::Release);
        assert_eq!(cell.try_write([3, 4]), Ok(usize::MAX - 1));
        assert_eq!(cell.try_write([5, 6]), Err(WriteError::VersionExhausted));
        let final_snapshot = cell.try_read(1).unwrap();
        assert_eq!(final_snapshot.version, usize::MAX - 1);
        assert_eq!(final_snapshot.value, [3, 4]);
        assert_eq!(cell.try_write([7, 8]), Err(WriteError::VersionExhausted));
    }
}

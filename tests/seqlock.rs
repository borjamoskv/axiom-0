#![cfg(target_has_atomic = "ptr")]

use std::{sync::Barrier, thread};

use micro_axiom_0::seqlock::{ReadError, SeqlockCell, WriteError};

#[test]
fn initial_snapshot_preserves_all_words_and_starts_at_version_zero() {
    let initial = [0_usize, 1, usize::MAX, 17];
    let cell = SeqlockCell::new(initial);
    let snapshot = cell.try_read(1).unwrap();

    assert_eq!(snapshot.value, initial);
    assert_eq!(snapshot.version, 0);
    assert_eq!(snapshot.attempts, 1);
}

#[test]
fn zero_read_budget_does_not_attempt_even_an_uncontended_read() {
    let cell = SeqlockCell::new([13_usize]);

    assert!(matches!(
        cell.try_read(0),
        Err(ReadError::RetryBudgetExhausted { attempts: 0 })
    ));
    assert_eq!(cell.try_write([29]).unwrap(), 2);
    assert!(matches!(
        cell.try_read(0),
        Err(ReadError::RetryBudgetExhausted { attempts: 0 })
    ));
    let snapshot = cell.try_read(1).unwrap();
    assert_eq!(snapshot.value, [29]);
    assert_eq!(snapshot.version, 2);
}

#[test]
fn uncontended_reads_report_actual_attempts_instead_of_the_budget() {
    let cell = SeqlockCell::new([usize::MAX]);

    for budget in [1, 2, 17, usize::MAX] {
        let snapshot = cell.try_read(budget).unwrap();
        assert_eq!(snapshot.attempts, 1);
        assert_eq!(snapshot.version, 0);
        assert_eq!(snapshot.value, [usize::MAX]);
    }
}

#[test]
fn successful_writes_advance_the_version_by_two_even_for_equal_values() {
    let cell = SeqlockCell::new([0_usize; 3]);
    let values = [[7, 11, 13], [7, 11, 13], [usize::MAX, 0, 5]];

    for (index, value) in values.into_iter().enumerate() {
        let expected_version = 2 * (index + 1);
        assert_eq!(cell.try_write(value).unwrap(), expected_version);
        let snapshot = cell.try_read(1).unwrap();
        assert_eq!(snapshot.value, value);
        assert_eq!(snapshot.version, expected_version);
        assert_eq!(snapshot.attempts, 1);
    }
}

#[test]
fn an_empty_payload_still_has_versioned_commits_and_a_read_budget() {
    let cell = SeqlockCell::<usize, 0>::new([]);

    assert!(matches!(
        cell.try_read(0),
        Err(ReadError::RetryBudgetExhausted { attempts: 0 })
    ));
    let initial = cell.try_read(1).unwrap();
    assert_eq!(initial.value, []);
    assert_eq!(initial.version, 0);
    assert_eq!(initial.attempts, 1);

    for expected_version in [2, 4, 6] {
        assert_eq!(cell.try_write([]).unwrap(), expected_version);
        let snapshot = cell.try_read(3).unwrap();
        assert_eq!(snapshot.value, []);
        assert_eq!(snapshot.version, expected_version);
        assert_eq!(snapshot.attempts, 1);
    }
}

macro_rules! integer_round_trip {
    ($name:ident, $ty:ty) => {
        #[test]
        fn $name() {
            let initial = [<$ty>::MIN, 0, <$ty>::MAX];
            let updated = [<$ty>::MAX, <$ty>::MIN, 1];
            let cell = SeqlockCell::<$ty, 3>::new(initial);

            let snapshot = cell.try_read(1).unwrap();
            assert_eq!(snapshot.value, initial);
            assert_eq!(snapshot.version, 0);
            assert_eq!(cell.try_write(updated).unwrap(), 2);
            let snapshot = cell.try_read(1).unwrap();
            assert_eq!(snapshot.value, updated);
            assert_eq!(snapshot.version, 2);
            assert_eq!(snapshot.attempts, 1);
        }
    };
}

#[cfg(target_has_atomic = "8")]
integer_round_trip!(u8_preserves_boundary_values, u8);
#[cfg(target_has_atomic = "8")]
integer_round_trip!(i8_preserves_boundary_values, i8);
#[cfg(target_has_atomic = "16")]
integer_round_trip!(u16_preserves_boundary_values, u16);
#[cfg(target_has_atomic = "16")]
integer_round_trip!(i16_preserves_boundary_values, i16);
#[cfg(target_has_atomic = "32")]
integer_round_trip!(u32_preserves_boundary_values, u32);
#[cfg(target_has_atomic = "32")]
integer_round_trip!(i32_preserves_boundary_values, i32);
#[cfg(target_has_atomic = "64")]
integer_round_trip!(u64_preserves_boundary_values, u64);
#[cfg(target_has_atomic = "64")]
integer_round_trip!(i64_preserves_boundary_values, i64);
integer_round_trip!(usize_preserves_boundary_values, usize);
integer_round_trip!(isize_preserves_boundary_values, isize);

#[cfg(target_has_atomic = "8")]
#[test]
fn bool_preserves_both_values() {
    let cell = SeqlockCell::new([true, false, true]);

    assert_eq!(cell.try_read(1).unwrap().value, [true, false, true]);
    assert_eq!(cell.try_write([false, true, false]).unwrap(), 2);
    let snapshot = cell.try_read(1).unwrap();
    assert_eq!(snapshot.value, [false, true, false]);
    assert_eq!(snapshot.version, 2);
    assert_eq!(snapshot.attempts, 1);
}

fn payload(ticket: usize) -> [usize; 4] {
    [
        ticket,
        !ticket,
        ticket.wrapping_mul(37),
        ticket.rotate_left(13),
    ]
}

#[test]
fn concurrent_snapshots_match_complete_commits_and_versions() {
    const WRITERS: usize = 4;
    const READERS: usize = 4;
    const WRITES_PER_WRITER: usize = 2_000;
    const READS_PER_READER: usize = 4_000;
    const READ_BUDGET: usize = 7;

    let cell = SeqlockCell::new(payload(0));
    let start = Barrier::new(WRITERS + READERS);

    let (writer_results, reader_results) = thread::scope(|scope| {
        let writers: Vec<_> = (0..WRITERS)
            .map(|writer| {
                let cell = &cell;
                let start = &start;
                scope.spawn(move || {
                    let mut commits = Vec::new();
                    let mut contentions = 0;
                    start.wait();
                    for iteration in 0..WRITES_PER_WRITER {
                        let value = payload(writer * WRITES_PER_WRITER + iteration + 1);
                        match cell.try_write(value) {
                            Ok(version) => commits.push((version, value)),
                            Err(WriteError::Contended) => contentions += 1,
                            Err(WriteError::VersionExhausted) => {
                                panic!("small fixed workload cannot exhaust the version")
                            }
                        }
                    }
                    (commits, contentions)
                })
            })
            .collect();
        let readers: Vec<_> = (0..READERS)
            .map(|_| {
                let cell = &cell;
                let start = &start;
                scope.spawn(move || {
                    let mut snapshots = Vec::new();
                    let mut exhausted = 0;
                    let mut previous_version = 0;
                    start.wait();
                    for _ in 0..READS_PER_READER {
                        match cell.try_read(READ_BUDGET) {
                            Ok(snapshot) => {
                                assert!((1..=READ_BUDGET).contains(&snapshot.attempts));
                                assert_eq!(snapshot.version % 2, 0);
                                assert!(snapshot.version >= previous_version);
                                assert_eq!(snapshot.value, payload(snapshot.value[0]));
                                previous_version = snapshot.version;
                                snapshots.push((snapshot.version, snapshot.value));
                            }
                            Err(ReadError::RetryBudgetExhausted { attempts }) => {
                                assert_eq!(attempts, READ_BUDGET);
                                exhausted += 1;
                            }
                        }
                    }
                    (snapshots, exhausted)
                })
            })
            .collect();

        let writer_results: Vec<_> = writers
            .into_iter()
            .map(|writer| writer.join().unwrap())
            .collect();
        let reader_results: Vec<_> = readers
            .into_iter()
            .map(|reader| reader.join().unwrap())
            .collect();
        (writer_results, reader_results)
    });

    let successful_commits: usize = writer_results
        .iter()
        .map(|(commits, _)| commits.len())
        .sum();
    assert!(successful_commits > 0);
    let mut committed_values = vec![None; successful_commits + 1];
    committed_values[0] = Some(payload(0));

    for (commits, contentions) in writer_results {
        assert_eq!(commits.len() + contentions, WRITES_PER_WRITER);
        for (version, value) in commits {
            assert_eq!(version % 2, 0);
            assert!(version > 0);
            let slot = &mut committed_values[version / 2];
            assert!(slot.replace(value).is_none(), "duplicate committed version");
        }
    }
    assert!(committed_values.iter().all(Option::is_some));

    for (snapshots, exhausted) in reader_results {
        assert_eq!(snapshots.len() + exhausted, READS_PER_READER);
        for (version, value) in snapshots {
            assert_eq!(Some(value), committed_values[version / 2]);
        }
    }

    let final_snapshot = cell.try_read(1).unwrap();
    assert_eq!(final_snapshot.attempts, 1);
    assert_eq!(final_snapshot.version, 2 * successful_commits);
    assert_eq!(
        Some(final_snapshot.value),
        committed_values[successful_commits]
    );
}

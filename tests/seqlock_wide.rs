#![cfg(target_has_atomic = "ptr")]

use std::{sync::Barrier, thread};

use micro_axiom_0::seqlock::{ReadError, SeqlockCell, WriteError};

// Cross many cache lines and end with a partial line instead of testing only
// the first few atomic words or a conveniently aligned payload length.
const WORDS: usize = 257;

fn payload(ticket: usize) -> [usize; WORDS] {
    std::array::from_fn(|index| {
        if index == 0 {
            ticket
        } else {
            // For each position this is a bijection of ticket: a word from a
            // different attempted write cannot accidentally satisfy the oracle.
            ticket
                .wrapping_mul(index.wrapping_mul(2).wrapping_add(1))
                .rotate_left((index % usize::BITS as usize) as u32)
                ^ index.wrapping_mul(37)
        }
    })
}

#[test]
fn wide_snapshots_match_the_complete_payload_of_the_reported_commit() {
    const WRITERS: usize = 4;
    const READERS: usize = 3;
    const WRITES_PER_WRITER: usize = 256;
    const READS_PER_READER: usize = 512;
    const READ_BUDGET: usize = 3;

    let cell = SeqlockCell::new(payload(0));
    let initial = cell.try_read(1).unwrap();
    assert_eq!(initial.value, payload(0));
    assert_eq!(initial.version, 0);
    assert_eq!(initial.attempts, 1);
    let start = Barrier::new(WRITERS + READERS);

    let (writer_results, reader_results) = thread::scope(|scope| {
        let writers: Vec<_> = (0..WRITERS)
            .map(|writer| {
                let cell = &cell;
                let start = &start;
                scope.spawn(move || {
                    let mut commits = Vec::with_capacity(WRITES_PER_WRITER);
                    let mut contentions = 0;
                    start.wait();
                    for attempt in 0..WRITES_PER_WRITER {
                        let ticket = writer * WRITES_PER_WRITER + attempt + 1;
                        match cell.try_write(payload(ticket)) {
                            Ok(version) => commits.push((version, ticket)),
                            Err(WriteError::Contended) => contentions += 1,
                            Err(WriteError::VersionExhausted) => {
                                panic!("bounded workload cannot exhaust the version")
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
                    let mut snapshots = Vec::with_capacity(READS_PER_READER);
                    let mut exhausted = 0;
                    start.wait();
                    for _ in 0..READS_PER_READER {
                        match cell.try_read(READ_BUDGET) {
                            Ok(snapshot) => {
                                assert!((1..=READ_BUDGET).contains(&snapshot.attempts));
                                assert_eq!(snapshot.version % 2, 0);
                                let ticket = snapshot.value[0];
                                assert_eq!(snapshot.value, payload(ticket));
                                // All words were checked above; retain the
                                // ticket to verify the version after writers join.
                                snapshots.push((snapshot.version, ticket));
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
    let mut committed_tickets = vec![None; successful_commits + 1];
    committed_tickets[0] = Some(0);
    for (commits, contentions) in writer_results {
        assert_eq!(commits.len() + contentions, WRITES_PER_WRITER);
        for (version, ticket) in commits {
            assert_eq!(version % 2, 0);
            assert!(version > 0);
            assert!(version / 2 <= successful_commits);
            assert!(
                committed_tickets[version / 2].replace(ticket).is_none(),
                "two writes reported the same committed version"
            );
        }
    }
    assert!(committed_tickets.iter().all(Option::is_some));

    for (snapshots, exhausted) in reader_results {
        assert_eq!(snapshots.len() + exhausted, READS_PER_READER);
        for (version, ticket) in snapshots {
            assert_eq!(committed_tickets.get(version / 2), Some(&Some(ticket)));
        }
    }

    // Scheduling need not produce a successful overlapping read. Always check
    // the last concurrent commit after quiescence and a fresh wide replacement.
    let last = cell.try_read(1).unwrap();
    assert_eq!(last.attempts, 1);
    assert_eq!(last.version, successful_commits * 2);
    assert_eq!(
        last.value,
        payload(committed_tickets[successful_commits].unwrap())
    );

    let final_value = payload(WRITERS * WRITES_PER_WRITER + 1);
    let final_version = (successful_commits + 1) * 2;
    assert_eq!(cell.try_write(final_value), Ok(final_version));
    let final_snapshot = cell.try_read(1).unwrap();
    assert_eq!(final_snapshot.attempts, 1);
    assert_eq!(final_snapshot.version, final_version);
    assert_eq!(final_snapshot.value, final_value);
}

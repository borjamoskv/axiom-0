#![cfg(target_has_atomic = "ptr")]

use std::{sync::mpsc, thread};

use micro_axiom_0::seqlock::SeqlockCell;

#[test]
fn a_reader_on_another_thread_observes_the_joined_writers_last_commit() {
    let cell = SeqlockCell::new([0_usize; 3]);
    let updates = [[1, 2, 3], [usize::MAX, 0, 17], [9, 8, 7]];

    thread::scope(|scope| {
        let writer_cell = &cell;
        let writer = scope.spawn(move || {
            let mut version = 0;
            for value in updates {
                version = writer_cell.try_write(value).unwrap();
            }
            version
        });
        let committed_version = writer.join().unwrap();
        assert_eq!(committed_version, 6);

        let reader_cell = &cell;
        let reader = scope.spawn(move || {
            let snapshot = reader_cell.try_read(1).unwrap();
            assert_eq!(snapshot.value, updates[2]);
            assert_eq!(snapshot.version, committed_version);
            assert_eq!(snapshot.attempts, 1);
        });
        reader.join().unwrap();
    });
}

#[test]
fn every_acknowledged_handoff_exposes_its_matching_value_and_version() {
    let cell = SeqlockCell::new([0_usize; 3]);
    let updates = [[1, 2, 3], [1, 2, 3], [usize::MAX, 0, 17], [9, 8, 7]];
    let (published, commits) = mpsc::sync_channel(0);
    let (acknowledged, acknowledgements) = mpsc::sync_channel(0);

    thread::scope(|scope| {
        let writer_cell = &cell;
        let writer = scope.spawn(move || {
            for value in updates {
                let version = writer_cell.try_write(value).unwrap();
                published.send(version).unwrap();
                // The reader finishes before the next write can start.
                acknowledgements.recv().unwrap();
            }
        });

        let reader_cell = &cell;
        let reader = scope.spawn(move || {
            for (index, value) in updates.into_iter().enumerate() {
                let committed_version = commits.recv().unwrap();
                assert_eq!(committed_version, 2 * (index + 1));

                let snapshot = reader_cell.try_read(1).unwrap();
                assert_eq!(snapshot.value, value);
                assert_eq!(snapshot.version, committed_version);
                assert_eq!(snapshot.attempts, 1);
                acknowledged.send(()).unwrap();
            }
        });

        writer.join().unwrap();
        reader.join().unwrap();
    });
}

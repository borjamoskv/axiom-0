//! Finite reads-from model of the atomic-word seqlock protocol.
//!
//! Scope: two successful, serialized writers replace two words in the same order;
//! one reader performs one attempt whose first sequence load is even. We enumerate
//! all 3 * 3 * 3 * 5 = 135 choices of read sources, including older modifications.
//! Each payload value is a generation tag, so mixed generations are observable.
//!
//! This independent model does not execute or import the implementation. It checks
//! a happens-before graph, four per-object coherence rules, and the RMW predecessor
//! rule for this finite history. It is not a complete C++/Rust memory-model checker,
//! a proof for arbitrary histories, or a test of the implementation's orderings.
//! It does not model counter wrap, scheduling, real-time freshness, or progress.
//!
//! Rules: <https://eel.is/c++draft/intro.races> and
//! <https://eel.is/c++draft/atomics.order>. Rust's documented atomic model follows
//! C++20: <https://doc.rust-lang.org/std/sync/atomic/index.html>.

const EVENT_COUNT: usize = 15;
const SEQUENCE: usize = 0;
const FIRST_WORD: usize = 1;
const SECOND_WORD: usize = 2;

const INITIAL_SEQUENCE: usize = 0;
const INITIAL_FIRST: usize = 1;
const INITIAL_SECOND: usize = 2;
const BEGIN_FIRST: usize = 3;
const WRITE_FIRST_FIRST: usize = 4;
const WRITE_FIRST_SECOND: usize = 5;
const COMMIT_FIRST: usize = 6;
const BEGIN_SECOND: usize = 7;
const WRITE_SECOND_FIRST: usize = 8;
const WRITE_SECOND_SECOND: usize = 9;
const COMMIT_SECOND: usize = 10;
const READ_BEFORE: usize = 11;
const READ_FIRST: usize = 12;
const READ_SECOND: usize = 13;
const READ_AFTER: usize = 14;

const SEQUENCE_WRITES: [usize; 5] = [
    INITIAL_SEQUENCE,
    BEGIN_FIRST,
    COMMIT_FIRST,
    BEGIN_SECOND,
    COMMIT_SECOND,
];
const FIRST_WRITES: [usize; 3] = [INITIAL_FIRST, WRITE_FIRST_FIRST, WRITE_SECOND_FIRST];
const SECOND_WRITES: [usize; 3] = [INITIAL_SECOND, WRITE_FIRST_SECOND, WRITE_SECOND_SECOND];

#[derive(Clone, Copy)]
enum PayloadOrdering {
    ReleaseAcquire,
    Relaxed,
}

#[derive(Clone, Copy)]
struct Event {
    object: usize,
    /// Position in this object's modification order, if this event writes.
    modification: Option<usize>,
    /// Event whose modification supplies this event's read, including CAS reads.
    source: Option<usize>,
    release: bool,
    acquire: bool,
}

impl Event {
    const fn write(object: usize, modification: usize, release: bool) -> Self {
        Self {
            object,
            modification: Some(modification),
            source: None,
            release,
            acquire: false,
        }
    }

    const fn read(object: usize, source: usize, acquire: bool) -> Self {
        Self {
            object,
            modification: None,
            source: Some(source),
            release: false,
            acquire,
        }
    }

    const fn cas(modification: usize, source: usize) -> Self {
        Self {
            object: SEQUENCE,
            modification: Some(modification),
            source: Some(source),
            release: true,
            acquire: true,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Observation {
    before: usize,
    words: [usize; 2],
    after: usize,
}

impl Observation {
    fn accepted(self) -> bool {
        self.before % 2 == 0 && self.before == self.after
    }

    fn matches_version(self) -> bool {
        self.words == [self.before / 2; 2]
    }
}

type HappensBefore = [[bool; EVENT_COUNT]; EVENT_COUNT];

fn sequence(hb: &mut HappensBefore, events: &[usize]) {
    for pair in events.windows(2) {
        hb[pair[0]][pair[1]] = true;
    }
}

fn close_transitively(hb: &mut HappensBefore) {
    for through in 0..EVENT_COUNT {
        // Copies keep the relation's update independent of borrow traversal.
        let successors = hb[through];
        for row in hb.iter_mut() {
            if row[through] {
                for (reachable, successor) in row.iter_mut().zip(successors) {
                    *reachable |= successor;
                }
            }
        }
    }
}

fn allowed(observation: Observation, ordering: PayloadOrdering) -> bool {
    let payload_ra = matches!(ordering, PayloadOrdering::ReleaseAcquire);
    let events = [
        Event::write(SEQUENCE, 0, false),
        Event::write(FIRST_WORD, 0, false),
        Event::write(SECOND_WORD, 0, false),
        Event::cas(1, INITIAL_SEQUENCE),
        Event::write(FIRST_WORD, 1, payload_ra),
        Event::write(SECOND_WORD, 1, payload_ra),
        Event::write(SEQUENCE, 2, true),
        Event::cas(3, COMMIT_FIRST),
        Event::write(FIRST_WORD, 2, payload_ra),
        Event::write(SECOND_WORD, 2, payload_ra),
        Event::write(SEQUENCE, 4, true),
        Event::read(SEQUENCE, SEQUENCE_WRITES[observation.before], true),
        Event::read(FIRST_WORD, FIRST_WRITES[observation.words[0]], payload_ra),
        Event::read(SECOND_WORD, SECOND_WRITES[observation.words[1]], payload_ra),
        Event::read(SEQUENCE, SEQUENCE_WRITES[observation.after], true),
    ];
    let mut hb = [[false; EVENT_COUNT]; EVENT_COUNT];

    // Synthetic initial writes are sequenced and safely published to all threads.
    sequence(&mut hb, &[INITIAL_SEQUENCE, INITIAL_FIRST, INITIAL_SECOND]);
    for row in hb.iter_mut().take(3) {
        row[BEGIN_FIRST..].fill(true);
    }
    sequence(
        &mut hb,
        &[
            BEGIN_FIRST,
            WRITE_FIRST_FIRST,
            WRITE_FIRST_SECOND,
            COMMIT_FIRST,
        ],
    );
    sequence(
        &mut hb,
        &[
            BEGIN_SECOND,
            WRITE_SECOND_FIRST,
            WRITE_SECOND_SECOND,
            COMMIT_SECOND,
        ],
    );
    sequence(&mut hb, &[READ_BEFORE, READ_FIRST, READ_SECOND, READ_AFTER]);

    for (reader, event) in events.iter().enumerate() {
        if let Some(source) = event.source {
            let origin = events[source];
            if origin.object != event.object || origin.modification.is_none() {
                return false;
            }
            // A successful RMW reads its immediately preceding modification.
            if let Some(rank) = event.modification {
                if origin.modification.unwrap() + 1 != rank {
                    return false;
                }
            }
            if origin.release && event.acquire {
                hb[source][reader] = true;
            }
        }
    }
    // Release-sequence ancestry adds no missing HB edge in this history: the
    // second CAS already acquires the first commit and is itself a release.
    close_transitively(&mut hb);

    for (index, event) in events.iter().enumerate() {
        if hb[index][index] {
            return false;
        }
        // An atomic load cannot read a modification that happens after the load.
        if event.source.is_some_and(|source| hb[index][source]) {
            return false;
        }
    }

    for (a, earlier) in events.iter().enumerate() {
        for (b, later) in events.iter().enumerate() {
            if earlier.object != later.object || !hb[a][b] {
                continue;
            }
            let earlier_read = earlier
                .source
                .map(|source| events[source].modification.unwrap());
            let later_read = later
                .source
                .map(|source| events[source].modification.unwrap());
            // Write-write: HB-ordered writes respect modification order.
            if let (Some(x), Some(y)) = (earlier.modification, later.modification) {
                if x >= y {
                    return false;
                }
            }
            // Read-read: a later read cannot move backwards in modification order.
            if let (Some(x), Some(y)) = (earlier_read, later_read) {
                if x > y {
                    return false;
                }
            }
            // Read-write: the observed write precedes any HB-later write.
            if let (Some(x), Some(y)) = (earlier_read, later.modification) {
                if x >= y {
                    return false;
                }
            }
            // Write-read: a read cannot precede an HB-earlier write in MO.
            if let (Some(x), Some(y)) = (earlier.modification, later_read) {
                if x > y {
                    return false;
                }
            }
        }
    }
    true
}

fn observations() -> impl Iterator<Item = Observation> {
    [0, 2, 4].into_iter().flat_map(|before| {
        (0..3).flat_map(move |first| {
            (0..3).flat_map(move |second| {
                (0..5).map(move |after| Observation {
                    before,
                    words: [first, second],
                    after,
                })
            })
        })
    })
}

#[test]
fn release_acquire_payload_rejects_every_accepted_incoherent_candidate() {
    let mut candidates = 0;
    let mut permitted = 0;
    let mut accepted_generations = [false; 3];
    for observation in observations() {
        candidates += 1;
        if allowed(observation, PayloadOrdering::ReleaseAcquire) {
            permitted += 1;
            if observation.accepted() {
                assert!(observation.matches_version(), "{observation:?}");
                accepted_generations[observation.before / 2] = true;
            }
        }
    }
    assert_eq!(candidates, 135);
    assert!(permitted > 3 && permitted < candidates);
    assert_eq!(accepted_generations, [true; 3]);
}

#[test]
fn model_allows_old_coherent_observations_and_rejected_mixed_attempts() {
    // The second writer exists, but no external HB edge forces the reader to
    // observe it. Older completed versions remain legal observations.
    for version in [0, 2] {
        let old = Observation {
            before: version,
            words: [version / 2; 2],
            after: version,
        };
        assert!(allowed(old, PayloadOrdering::ReleaseAcquire));
        assert!(old.accepted());
    }
    let mixed = Observation {
        before: 0,
        words: [0, 1],
        after: 2,
    };
    assert!(allowed(mixed, PayloadOrdering::ReleaseAcquire));
    assert!(!mixed.accepted());
}

#[test]
fn relaxed_payload_has_an_accepted_mixed_reads_from_witness() {
    // Keep every sequence ordering unchanged. Reading one new payload word no
    // longer carries its writer's odd sequence into HB before the final load.
    let witness = Observation {
        before: 0,
        words: [0, 1],
        after: 0,
    };
    assert!(witness.accepted());
    assert!(!witness.matches_version());
    assert!(allowed(witness, PayloadOrdering::Relaxed));
    assert!(!allowed(witness, PayloadOrdering::ReleaseAcquire));

    let violations = observations()
        .filter(|observation| {
            observation.accepted()
                && !observation.matches_version()
                && allowed(*observation, PayloadOrdering::Relaxed)
        })
        .count();
    assert!(violations > 0);
}

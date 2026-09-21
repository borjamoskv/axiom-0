use micro_axiom_0::ast::{Ast, Expr, Level, Quantity};
use micro_axiom_0::eval::Value;
use micro_axiom_0::turbine::{AtomicElabSnapshot, ElabStatus, TurbineEngine, TypeTag};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

#[test]
fn test_turbine_atomic_snapshot_codec() {
    let snap = AtomicElabSnapshot {
        status: ElabStatus::CertifiedValid,
        type_tag: TypeTag::Universe,
        level: 42,
        quantity: Quantity::One,
    };

    let words = snap.to_words();
    let decoded = AtomicElabSnapshot::from_words(words);
    assert_eq!(snap, decoded);
    assert_eq!(decoded.to_value(), Some(Value::Universe(42)));
}

#[test]
fn test_turbine_concurrent_readers_and_writers() {
    let turbine = Arc::new(TurbineEngine::new(4));
    let stop = Arc::new(AtomicBool::new(false));

    let mut handles = Vec::new();

    let mut ast = Ast::new();
    let dummy = ast.push(Expr::Unit).unwrap();

    // Spawn 4 concurrent reader threads
    for _ in 0..4 {
        let t = Arc::clone(&turbine);
        let s = Arc::clone(&stop);
        handles.push(thread::spawn(move || {
            let mut read_count = 0;
            while !s.load(Ordering::Relaxed) {
                if let Ok(snap) = t.try_get_cached(dummy, 16) {
                    let decoded = AtomicElabSnapshot::from_words(snap.value);
                    if decoded.status == ElabStatus::CertifiedValid {
                        assert_eq!(decoded.type_tag, TypeTag::Universe);
                        // Level must match the published level
                        assert!(decoded.level <= 10_000);
                        read_count += 1;
                    }
                }
            }
            read_count
        }));
    }

    // Writer thread updating versions
    {
        let t = Arc::clone(&turbine);
        for i in 1..=10_000 {
            let snap = AtomicElabSnapshot {
                status: ElabStatus::CertifiedValid,
                type_tag: TypeTag::Universe,
                level: i,
                quantity: Quantity::Omega,
            };
            assert!(t.publish(dummy, snap).is_ok());
            if i % 500 == 0 {
                thread::yield_now();
            }
        }
        stop.store(true, Ordering::Release);
    }

    let mut total_reads = 0;
    for h in handles {
        let count = h.join().expect("Reader thread panicked");
        total_reads += count;
    }
    assert!(total_reads > 0, "Readers observed zero commits in total");
}

#[test]
fn test_turbine_parallel_batch_elaboration() {
    let mut ast = Ast::new();
    let mut roots = Vec::new();

    // Create 64 independent expressions in the AST
    for i in 0..64 {
        let u = ast.push(Expr::Universe(i)).unwrap();
        roots.push(u);
    }

    let turbine = TurbineEngine::for_ast(&ast);
    assert!(turbine.slot_count() >= 64);

    let results = turbine.elaborate_batch_parallel(&ast, &roots, &[]);
    assert_eq!(results.len(), 64);

    for (i, res) in results.into_iter().enumerate() {
        let elab = res.expect("Elaboration failed");
        assert_eq!(elab.ty, Value::Universe((i + 1) as u32));

        // Check that the turbine atomically recorded the certification
        let snap = turbine
            .try_get_cached(roots[i], 8)
            .expect("Cached snapshot missing");
        let decoded = AtomicElabSnapshot::from_words(snap.value);
        assert_eq!(decoded.status, ElabStatus::CertifiedValid);
        assert_eq!(decoded.type_tag, TypeTag::Universe);
        assert_eq!(decoded.level, (i + 1));
    }
}

#[test]
fn test_turbine_parallel_batch_checking() {
    let mut ast = Ast::new();
    let mut roots = Vec::new();

    // Construct 32 identity lambdas: fn :^1 x -> x
    for _ in 0..32 {
        let var0 = ast.push(Expr::Var(Level(0))).unwrap();
        let lam = ast
            .push(Expr::Lambda {
                quantity: Quantity::One,
                body: var0,
            })
            .unwrap();
        roots.push(lam);
    }

    // Expected type: fn :^1 (x: type 0) -> type 0
    let u0 = ast.push(Expr::Universe(0)).unwrap();
    let u0_cod = ast.push(Expr::Universe(0)).unwrap();
    let pi_ty = ast
        .push(Expr::Pi {
            quantity: Quantity::One,
            domain: u0,
            codomain: u0_cod,
        })
        .unwrap();
    let expected_val = micro_axiom_0::eval::eval(&ast, pi_ty, &[], None);

    let turbine = TurbineEngine::for_ast(&ast);
    let results = turbine.check_batch_parallel(&ast, &roots, expected_val, &[]);
    assert_eq!(results.len(), 32);

    for (i, res) in results.into_iter().enumerate() {
        let elab = res.expect("Check failed");
        assert!(elab.usages.is_empty());

        let snap = turbine
            .try_get_cached(roots[i], 8)
            .expect("Cached snapshot missing");
        let decoded = AtomicElabSnapshot::from_words(snap.value);
        assert_eq!(decoded.status, ElabStatus::CertifiedValid);
    }
}

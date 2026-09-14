use micro_axiom_0::{
    ast::{Ast, Expr, Quantity},
    elaborator,
};

#[test]
fn identical_offsets_in_distinct_arenas_are_not_identical_handles() {
    let mut left = Ast::new();
    let mut right = Ast::new();
    assert_ne!(left.unit_type(), right.unit_type());
    let a = left.push(Expr::Unit).unwrap();
    let b = right.push(Expr::Unit).unwrap();
    assert_ne!(a, b);
    assert!(left.expr(b).is_err());
    assert!(right.expr(a).is_err());
    assert!(left.ty(right.unit_type()).is_err());
}

#[test]
fn foreign_handles_are_rejected_before_mutating_the_destination() {
    let mut left = Ast::new();
    let mut right = Ast::new();
    let a = left.push(Expr::Unit).unwrap();
    let b = right.push(Expr::Unit).unwrap();
    let count = right.expression_count();
    assert!(
        right
            .push(Expr::Lambda {
                quantity: Quantity::Zero,
                body: a
            })
            .is_err()
    );
    assert!(
        right
            .push(Expr::App {
                function: b,
                argument: a
            })
            .is_err()
    );
    assert!(
        right
            .push(Expr::App {
                function: a,
                argument: b
            })
            .is_err()
    );
    assert!(
        right
            .push(Expr::Ann {
                term: a,
                ty: right.unit_type()
            })
            .is_err()
    );
    assert!(
        right
            .push(Expr::Ann {
                term: b,
                ty: left.unit_type()
            })
            .is_err()
    );
    assert_eq!(right.expression_count(), count);
    assert!(
        right
            .function_type(Quantity::One, left.unit_type(), right.unit_type())
            .is_err()
    );
    assert!(
        right
            .function_type(Quantity::One, right.unit_type(), left.unit_type())
            .is_err()
    );
}

#[test]
fn public_elaboration_never_reinterprets_a_foreign_root_or_type() {
    let mut left = Ast::new();
    let mut right = Ast::new();
    let a = left.push(Expr::Unit).unwrap();
    let b = right.push(Expr::Unit).unwrap();
    assert!(elaborator::synthesize(&right, a).is_err());
    assert!(elaborator::check(&right, a, right.unit_type()).is_err());
    assert!(elaborator::check(&right, b, left.unit_type()).is_err());
    assert!(elaborator::synthesize(&right, b).is_ok());
}

#[test]
fn dropping_an_arena_does_not_make_its_handles_valid_again() {
    let old = {
        let mut ast = Ast::new();
        ast.push(Expr::Unit).unwrap()
    };
    let mut next = Ast::default();
    let current = next.push(Expr::Unit).unwrap();
    assert_ne!(old, current);
    assert!(next.expr(old).is_err());
}

#[test]
fn concurrent_arena_construction_assigns_distinct_identities() {
    let identities = std::thread::scope(|scope| {
        let tasks: Vec<_> = (0..32)
            .map(|_| scope.spawn(|| Ast::new().unit_type()))
            .collect();
        tasks
            .into_iter()
            .map(|t| t.join().unwrap())
            .collect::<std::collections::BTreeSet<_>>()
    });
    assert_eq!(identities.len(), 32);
}

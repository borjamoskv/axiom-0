use micro_axiom_0::ast::{Ast, Expr, Quantity};
use micro_axiom_0::elaborator::{Error, check, synthesize};
use micro_axiom_0::eval::Value;

#[test]
fn maximum_universe_cannot_be_synthesized_or_checked() {
    let mut ast = Ast::new();
    let universe = ast.push(Expr::Universe(u32::MAX)).unwrap();
    let expected_error = Error::UniverseOverflow {
        expr: universe,
        level: u32::MAX,
    };

    assert_eq!(synthesize(&ast, universe, &[]).unwrap_err(), expected_error);
    // Neither equality nor cumulativity may bypass the missing successor.
    for expected_level in [0, u32::MAX - 1, u32::MAX] {
        assert_eq!(
            check(&ast, universe, Value::Universe(expected_level), &[]).unwrap_err(),
            expected_error,
        );
    }
}

#[test]
fn penultimate_universe_has_the_maximum_universe_as_its_type() {
    let mut ast = Ast::new();
    let universe = ast.push(Expr::Universe(u32::MAX - 1)).unwrap();

    let synthesized = synthesize(&ast, universe, &[]).unwrap();
    assert_eq!(synthesized.ty, Value::Universe(u32::MAX));
    assert!(synthesized.usages.is_empty());

    let checked = check(&ast, universe, Value::Universe(u32::MAX), &[]).unwrap();
    assert_eq!(checked.ty, Value::Universe(u32::MAX));
    assert!(checked.usages.is_empty());
}

#[test]
fn cumulativity_accepts_equal_or_higher_universes_at_the_boundary() {
    let mut ast = Ast::new();
    let universe = ast.push(Expr::Universe(u32::MAX - 2)).unwrap();

    for expected_level in [u32::MAX - 1, u32::MAX] {
        let elaboration = check(&ast, universe, Value::Universe(expected_level), &[]).unwrap();
        assert_eq!(elaboration.ty, Value::Universe(expected_level));
        assert!(elaboration.usages.is_empty());
    }

    let unit_type = ast.push(Expr::UnitType).unwrap();
    let elaboration = check(&ast, unit_type, Value::Universe(u32::MAX), &[]).unwrap();
    assert_eq!(elaboration.ty, Value::Universe(u32::MAX));
}

#[test]
fn cumulativity_never_lowers_a_universe_at_the_boundary() {
    let mut ast = Ast::new();
    let universe = ast.push(Expr::Universe(u32::MAX - 1)).unwrap();

    for expected_level in [0, u32::MAX - 2, u32::MAX - 1] {
        assert!(matches!(
            check(&ast, universe, Value::Universe(expected_level), &[]),
            Err(Error::TypeMismatch { expr, .. }) if expr == universe
        ));
    }
}

#[test]
fn pi_formation_preserves_the_larger_boundary_universe() {
    for (domain_level, codomain_level) in [
        (u32::MAX - 1, 0),
        (0, u32::MAX - 1),
        (u32::MAX - 1, u32::MAX - 1),
    ] {
        let mut ast = Ast::new();
        let domain = ast.push(Expr::Universe(domain_level)).unwrap();
        let codomain = ast.push(Expr::Universe(codomain_level)).unwrap();
        let pi = ast
            .push(Expr::Pi { plicity: micro_axiom_0::ast::Plicity::Explicit,
                quantity: Quantity::Omega,
                domain,
                codomain,
            })
            .unwrap();

        let elaboration = synthesize(&ast, pi, &[]).unwrap();
        assert_eq!(elaboration.ty, Value::Universe(u32::MAX));
        assert!(elaboration.usages.is_empty());
    }
}

#[test]
fn pi_formation_propagates_overflow_in_either_component() {
    for overflowing_domain in [true, false] {
        let mut ast = Ast::new();
        let valid_type = ast.push(Expr::UnitType).unwrap();
        let overflowing_type = ast.push(Expr::Universe(u32::MAX)).unwrap();
        let (domain, codomain) = if overflowing_domain {
            (overflowing_type, valid_type)
        } else {
            (valid_type, overflowing_type)
        };
        let pi = ast
            .push(Expr::Pi { plicity: micro_axiom_0::ast::Plicity::Explicit,
                quantity: Quantity::Omega,
                domain,
                codomain,
            })
            .unwrap();

        assert_eq!(
            synthesize(&ast, pi, &[]).unwrap_err(),
            Error::UniverseOverflow {
                expr: overflowing_type,
                level: u32::MAX,
            },
        );
    }
}

#[test]
fn pi_formation_rejects_a_non_type_domain_or_codomain() {
    for invalid_domain in [true, false] {
        let mut ast = Ast::new();
        let valid_type = ast.push(Expr::UnitType).unwrap();
        let non_type = ast.push(Expr::Unit).unwrap();
        let (domain, codomain) = if invalid_domain {
            (non_type, valid_type)
        } else {
            (valid_type, non_type)
        };
        let pi = ast
            .push(Expr::Pi { plicity: micro_axiom_0::ast::Plicity::Explicit,
                quantity: Quantity::Omega,
                domain,
                codomain,
            })
            .unwrap();

        assert!(matches!(
            synthesize(&ast, pi, &[]),
            Err(Error::TypeMismatch { expr, .. }) if expr == non_type
        ));
    }
}

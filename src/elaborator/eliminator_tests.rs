use super::*;
use crate::ast::Command;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::turbine::TurbineEngine;

fn parse(source: &str) -> (Ast, ExprId) {
    let mut ast = Ast::new();
    let tokens = Lexer::new(source).tokenize_all().unwrap();
    let command = Parser::new(&tokens, &mut ast, &[]).parse_command().unwrap();
    let Command::Eval(root) = command else {
        panic!("expected an expression");
    };
    (ast, root)
}

fn assert_rejected(source: &str, expected: fn(&Error) -> bool) {
    let (ast, root) = parse(source);
    let turbine = TurbineEngine::for_ast(&ast);
    for result in [
        synthesize(&ast, root, &[]),
        synthesize_with_turbine(&ast, root, &[], &turbine),
    ] {
        let error = result.expect_err(source);
        assert!(expected(&error), "{source}: unexpected {error:?}");
    }
}

fn assert_evaluates(source: &str, ty: Value, value: Value) {
    let (ast, root) = parse(source);
    assert_eq!(synthesize(&ast, root, &[]).unwrap().ty, ty, "{source}");
    assert_eq!(eval(&ast, root, &[], None), value, "{source}");
    let turbine = TurbineEngine::for_ast(&ast);
    assert_eq!(
        synthesize_with_turbine(&ast, root, &[], &turbine)
            .unwrap()
            .ty,
        ty,
        "{source} with telemetry/inference enabled"
    );
    assert_eq!(eval(&ast, root, &[], Some(&turbine)), value, "{source}");
}

fn plicity_error(error: &Error) -> bool {
    matches!(error, Error::PlicityMismatch { .. })
}

fn type_error(error: &Error) -> bool {
    matches!(error, Error::TypeMismatch { .. })
}

fn function_error(error: &Error) -> bool {
    matches!(error, Error::ExpectedFunction { .. })
}

fn linear_usage_error(error: &Error) -> bool {
    matches!(
        error,
        Error::UsageMismatch {
            declared: Quantity::One,
            observed: Quantity::Zero,
            ..
        }
    )
}

#[test]
fn induction_rejects_implicit_motives() {
    for motive in ["fn {n} -> Nat", "(fn {n} -> Nat) : fn {n : Nat} -> type"] {
        assert_rejected(
            &format!("ind({motive}, Z, fn :^0 n -> fn :^1 ih -> S(ih), S(Z))"),
            plicity_error,
        );
    }
}

#[test]
fn induction_rejects_implicit_step_parameters() {
    for step in [
        "fn :^0 {n} -> fn :^1 ih -> S(ih)",
        "fn :^0 n -> fn :^1 {ih} -> S(ih)",
    ] {
        assert_rejected(&format!("ind(fn n -> Nat, Z, {step}, S(Z))"), plicity_error);
    }
}

#[test]
fn induction_rejects_discarded_linear_motive_parameters() {
    assert_rejected(
        "ind(fn :^1 n -> Nat, Z, fn :^0 n -> fn :^1 ih -> S(ih), S(Z))",
        linear_usage_error,
    );
}

#[test]
fn induction_checks_motive_codomain_before_base() {
    let source = "ind((fn n -> Z) : fn (n : Nat) -> Nat, Z, fn :^0 n -> fn :^1 ih -> S(ih), S(Z))";
    let (ast, root) = parse(source);
    let Expr::Ind { mot, .. } = ast.expr(root).unwrap() else {
        panic!("expected induction");
    };
    let turbine = TurbineEngine::for_ast(&ast);
    for result in [
        synthesize(&ast, root, &[]),
        synthesize_with_turbine(&ast, root, &[], &turbine),
    ] {
        assert!(
            matches!(result, Err(Error::TypeMismatch { expr, .. }) if expr == mot),
            "the malformed motive must be rejected before checking its base"
        );
    }
}

#[test]
fn induction_accepts_inline_and_annotated_motives() {
    for motive in [
        "fn n -> Nat",
        "(fn n -> Nat) : fn (n : Nat) -> type",
        "fn :^0 n -> Nat",
    ] {
        assert_evaluates(
            &format!("ind({motive}, Z, fn :^0 n -> fn :^1 ih -> S(ih), S(S(Z)))"),
            Value::NatType,
            Value::Succ(Box::new(Value::Succ(Box::new(Value::Zero)))),
        );
    }
}

#[test]
fn identity_elimination_requires_two_motive_parameters() {
    for source in ["J(Bool, True, refl(Z))", "J(fn b -> Bool, True, refl(Z))"] {
        assert_rejected(source, function_error);
    }
}

#[test]
fn identity_elimination_checks_motive_domains_and_endpoints() {
    for motive_type in [
        "fn (b : Bool) -> fn (p : Bool) -> type",
        "fn (b : Nat) -> fn (p : Bool) -> type",
        "fn (b : Nat) -> fn (p : Id(Nat, S(Z), b)) -> type",
        "fn (b : Nat) -> fn (p : Id(Nat, Z, Z)) -> type",
    ] {
        assert_rejected(
            &format!("J((fn b -> fn p -> Bool) : {motive_type}, True, refl(Z))"),
            type_error,
        );
    }
}

#[test]
fn identity_elimination_rejects_implicit_motive_parameters() {
    for motive in [
        "fn {b} -> fn p -> Bool",
        "fn b -> fn {p} -> Bool",
        "(fn {b} -> fn p -> Bool) : fn {b : Nat} -> fn (p : Id(Nat, Z, b)) -> type",
        "(fn b -> fn {p} -> Bool) : fn (b : Nat) -> fn {p : Id(Nat, Z, b)} -> type",
    ] {
        assert_rejected(&format!("J({motive}, True, refl(Z))"), plicity_error);
    }
}

#[test]
fn identity_elimination_rejects_discarded_linear_motive_parameters() {
    for motive in ["fn :^1 b -> fn p -> Bool", "fn b -> fn :^1 p -> Bool"] {
        assert_rejected(&format!("J({motive}, True, refl(Z))"), linear_usage_error);
    }
}

#[test]
fn identity_elimination_accepts_inline_annotated_and_partial_motives() {
    for motive in [
        "fn b -> fn p -> Bool",
        "(fn b -> fn p -> Bool) : fn (b : Nat) -> fn (p : Id(Nat, Z, b)) -> type",
        "fn b -> ((fn p -> Bool) : fn (p : Id(Nat, Z, b)) -> type)",
    ] {
        assert_evaluates(
            &format!("J({motive}, True, refl(Z))"),
            Value::Bool,
            Value::True,
        );
    }
}

#[test]
fn identity_elimination_preserves_dependent_result_types() {
    assert_evaluates(
        "J(fn b -> fn p -> Id(Nat, b, b), refl(Z), refl(Z))",
        Value::IdType(
            Box::new(Value::NatType),
            Box::new(Value::Zero),
            Box::new(Value::Zero),
        ),
        Value::Refl(Box::new(Value::Zero)),
    );
}

#[test]
fn identity_elimination_accepts_a_context_bound_motive() {
    assert_evaluates(
        "((fn m -> fn base -> J(m, base, refl(Z))) : fn (m : fn (b : Nat) -> fn (p : Id(Nat, Z, b)) -> type) -> fn (base : m Z refl(Z)) -> m Z refl(Z)) ((fn b -> fn p -> Bool) : fn (b : Nat) -> fn (p : Id(Nat, Z, b)) -> type) True",
        Value::Bool,
        Value::True,
    );
}

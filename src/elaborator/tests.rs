use super::*;
use crate::ast::{Command, Plicity};
use crate::eval::Neutral;
use crate::lexer::Lexer;
use crate::parser::Parser;

fn parse(source: &str, globals: &[String]) -> (Ast, ExprId) {
    let mut ast = Ast::new();
    let tokens = Lexer::new(source).tokenize_all().unwrap();
    let command = Parser::new(&tokens, &mut ast, globals)
        .parse_command()
        .unwrap();
    let Command::Eval(root) = command else {
        panic!("expected an expression");
    };
    (ast, root)
}

fn assert_rejected(source: &str, expected: Plicity, found: Plicity) {
    let (ast, root) = parse(source, &[]);
    let matches_error = |result| {
        matches!(
            result,
            Err(Error::PlicityMismatch { expected: e, found: f, .. })
                if e == expected && f == found
        )
    };
    assert!(matches_error(synthesize(&ast, root, &[])), "{source}");
    let turbine = crate::turbine::TurbineEngine::for_ast(&ast);
    assert!(
        matches_error(synthesize_with_turbine(&ast, root, &[], &turbine)),
        "{source} with telemetry/inference enabled"
    );
}

#[test]
fn implicit_lambda_cannot_check_as_explicit() {
    assert_rejected(
        "(fn {x} -> x) : fn (x : Bool) -> Bool",
        Plicity::Explicit,
        Plicity::Implicit,
    );
}

#[test]
fn explicit_lambda_cannot_check_as_implicit() {
    assert_rejected(
        "(fn x -> x) : fn {x : Bool} -> Bool",
        Plicity::Implicit,
        Plicity::Explicit,
    );
}

#[test]
fn implicit_argument_cannot_apply_to_explicit_parameter() {
    assert_rejected(
        "((fn x -> x) : fn (x : Bool) -> Bool) {True}",
        Plicity::Explicit,
        Plicity::Implicit,
    );
}

#[test]
fn matching_parameter_and_argument_plicity_still_evaluate() {
    for source in [
        "((fn x -> x) : fn (x : Bool) -> Bool) True",
        "((fn {x} -> x) : fn {x : Bool} -> Bool) {True}",
        "((fn {A} -> fn :^1 x -> x) : fn {A : type} -> fn :^1 (x : A) -> A) {Bool} True",
    ] {
        let (ast, root) = parse(source, &[]);
        assert_eq!(synthesize(&ast, root, &[]).unwrap().ty, Value::Bool);
        assert_eq!(eval(&ast, root, &[], None), Value::True);
        let turbine = crate::turbine::TurbineEngine::for_ast(&ast);
        assert_eq!(
            synthesize_with_turbine(&ast, root, &[], &turbine)
                .unwrap()
                .ty,
            Value::Bool
        );
        assert_eq!(eval(&ast, root, &[], Some(&turbine)), Value::True);
    }
}

#[cfg(feature = "std")]
#[test]
fn inferred_implicit_arguments_still_evaluate() {
    let source = "((fn {A} -> fn :^1 x -> x) : fn {A : type} -> fn :^1 (x : A) -> A) True";
    let (ast, root) = parse(source, &[]);
    let turbine = crate::turbine::TurbineEngine::for_ast(&ast);
    let result = synthesize_with_turbine(&ast, root, &[], &turbine).unwrap();
    assert_eq!(turbine.force(&ast, result.ty), Value::Bool);
    assert_eq!(eval(&ast, root, &[], Some(&turbine)), Value::True);
}

fn assert_eta(plicity: Plicity) {
    let source = match plicity {
        Plicity::Explicit => "fn x -> f x",
        Plicity::Implicit => "fn {x} -> f {x}",
    };
    let (ast, root) = parse(source, &["f".into()]);
    let f = Value::Neutral(Neutral::Var(Level(0)));
    let eta = eval(&ast, root, core::slice::from_ref(&f), None);
    assert!(equiv(&ast, &eta, &f, 1));
    assert!(equiv(&ast, &f, &eta, 1));
}

#[test]
fn explicit_eta_equivalence_is_symmetric() {
    assert_eta(Plicity::Explicit);
}

#[test]
fn implicit_eta_equivalence_is_symmetric() {
    assert_eta(Plicity::Implicit);
}

#[cfg(feature = "std")]
#[test]
fn implicit_eta_unification_is_symmetric() {
    let (ast, root) = parse("fn {x} -> f {x}", &["f".into()]);
    let turbine = crate::turbine::TurbineEngine::for_ast(&ast);
    let f = Value::Neutral(Neutral::Var(Level(0)));
    let eta = eval(&ast, root, core::slice::from_ref(&f), Some(&turbine));
    assert!(Checker::unify(&ast, &eta, &f, 1, &turbine));
    assert!(Checker::unify(&ast, &f, &eta, 1, &turbine));
}

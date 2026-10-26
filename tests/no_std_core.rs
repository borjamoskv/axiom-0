//! Exercises the allocation-only public kernel APIs against a hosted test harness.

use micro_axiom_0::ast::{Ast, Command, ExprId};
use micro_axiom_0::elaborator::{self, Error};
use micro_axiom_0::erasure::{self, RuntimeTerm};
use micro_axiom_0::eval::{self, Value};
use micro_axiom_0::lexer::Lexer;
use micro_axiom_0::parser::Parser;
use micro_axiom_0::turbine::{AtomicElabSnapshot, ElabStatus, TurbineEngine};

fn parse(source: &str) -> (Ast, ExprId) {
    let mut ast = Ast::new();
    let tokens = Lexer::new(source)
        .tokenize_all()
        .expect("valid test tokens");
    let command = Parser::new(&tokens, &mut ast, &[])
        .parse_command()
        .expect("valid test expression");
    let Command::Eval(root) = command else {
        panic!("expected an expression");
    };
    (ast, root)
}

#[test]
fn explicit_kernel_pipeline_preserves_erasure_and_atomic_telemetry() {
    let (ast, root) = parse("((fn :^0 x -> True) : fn :^0 (x : Nat) -> Bool) Z");
    let engine = TurbineEngine::for_ast(&ast);
    let elaboration = elaborator::synthesize_with_turbine(&ast, root, &[], &engine)
        .expect("explicit QTT checking works with the telemetry plane");
    assert_eq!(elaboration.ty, Value::Bool);
    assert_eq!(eval::eval(&ast, root, &[], Some(&engine)), Value::True);
    let erased = erasure::erase_expression(&ast, root).expect("erase the checked term");
    assert_eq!(erasure::eval_runtime(&erased, &[]), RuntimeTerm::Bool(true));
    let snapshot = engine.try_snapshot(root, 1).expect("coherent telemetry");
    assert_eq!(
        AtomicElabSnapshot::from_words(snapshot.value).status,
        ElabStatus::CertifiedValid
    );
}

#[test]
fn polymorphism_with_explicit_implicit_arguments_needs_no_hosted_inference() {
    let (ast, root) =
        parse("((fn {A} -> fn :^1 x -> x) : fn {A : type} -> fn :^1 (x : A) -> A) {Nat} S(Z)");
    let elaboration = elaborator::synthesize(&ast, root, &[]).expect("explicit polymorphism");
    assert_eq!(elaboration.ty, Value::NatType);
    assert_eq!(
        eval::eval(&ast, root, &[], None),
        Value::Succ(Box::new(Value::Zero))
    );
    let erased = erasure::erase_expression(&ast, root).expect("erase type arguments");
    assert_eq!(erasure::eval_runtime(&erased, &[]), RuntimeTerm::Nat(1));
}

#[test]
fn explicit_kernel_rejects_untyped_holes() {
    let (ast, root) = parse("?");
    assert!(matches!(
        elaborator::synthesize(&ast, root, &[]),
        Err(Error::TypeMismatch { .. })
    ));
}

#[test]
#[cfg(not(feature = "std"))]
fn telemetry_only_engine_cannot_certify_unresolved_holes() {
    let (ast, root) = parse("?");
    let engine = TurbineEngine::for_ast(&ast);
    assert!(matches!(
        elaborator::synthesize_with_turbine(&ast, root, &[], &engine),
        Err(Error::TypeMismatch { .. })
    ));
    let snapshot = engine.try_snapshot(root, 1).expect("initial telemetry");
    assert_eq!(
        AtomicElabSnapshot::from_words(snapshot.value).status,
        ElabStatus::Unelaborated
    );
}

#[test]
#[cfg(not(feature = "std"))]
fn implicit_inference_is_rejected_without_the_hosted_solver() {
    let (ast, root) = parse("((fn {A} -> fn x -> x) : fn {A : type} -> fn (x : A) -> A) True");
    let engine = TurbineEngine::for_ast(&ast);
    assert!(matches!(
        elaborator::synthesize_with_turbine(&ast, root, &[], &engine),
        Err(Error::TypeMismatch { .. })
    ));
}

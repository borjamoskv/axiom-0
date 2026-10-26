//! Semantic and native-code regressions for the six V1 audit findings.

use micro_axiom_0::ast::{Ast, Command, ExprId, Quantity};
#[cfg(feature = "std")]
use micro_axiom_0::ast::{Expr, MetaId, Plicity};
use micro_axiom_0::elaborator;
#[cfg(feature = "std")]
use micro_axiom_0::elaborator::Checker;
use micro_axiom_0::erasure::{self, CodegenError, RuntimeTerm};
#[cfg(feature = "std")]
use micro_axiom_0::eval::{self, Value};
use micro_axiom_0::lexer::Lexer;
use micro_axiom_0::parser::Parser;
#[cfg(feature = "std")]
use micro_axiom_0::turbine::TurbineEngine;
use std::path::PathBuf;
use std::process::Command as Process;
use std::sync::atomic::{AtomicUsize, Ordering};

fn parse(source: &str) -> (Ast, ExprId) {
    let mut ast = Ast::new();
    let tokens = Lexer::new(source)
        .tokenize_all()
        .expect("valid regression tokens");
    let command = Parser::new(&tokens, &mut ast, &[])
        .parse_command()
        .expect("valid regression syntax");
    let Command::Eval(root) = command else {
        panic!("expected an expression");
    };
    (ast, root)
}

fn checked_erasure(source: &str) -> RuntimeTerm {
    let (ast, root) = parse(source);
    elaborator::synthesize(&ast, root, &[]).expect("well-typed regression");
    erasure::erase_expression(&ast, root).expect("erasure succeeds")
}

#[test]
#[cfg(feature = "std")]
fn occurs_check_follows_solved_metas_and_closure_bodies() {
    let mut ast = Ast::new();
    let turbine = TurbineEngine::new(0);
    let m = turbine.new_meta();
    let n = turbine.new_meta();
    let meta = Value::Meta(m, vec![]);
    let other = Value::Meta(n, vec![]);
    assert!(Checker::unify(&ast, &meta, &other, 0, &turbine));
    assert!(!Checker::unify(
        &ast,
        &other,
        &Value::Succ(Box::new(meta)),
        0,
        &turbine
    ));
    let body = ast.push(Expr::Meta(n)).expect("meta body");
    let closure = eval::Closure { env: vec![], body };
    let lambda = Value::Lam(Plicity::Explicit, Quantity::Omega, closure);
    assert!(!Checker::unify(&ast, &other, &lambda, 0, &turbine));
    assert!(Checker::unify(&ast, &other, &Value::NatType, 0, &turbine));
}

#[test]
#[cfg(feature = "std")]
fn concurrent_occurs_checks_cannot_commit_a_mutual_cycle() {
    let ast = Ast::new();
    for _ in 0..64 {
        let turbine = TurbineEngine::new(0);
        let m = Value::Meta(turbine.new_meta(), vec![]);
        let n = Value::Meta(turbine.new_meta(), vec![]);
        let barrier = std::sync::Barrier::new(2);
        let (left, right) = std::thread::scope(|scope| {
            let left = scope.spawn(|| {
                barrier.wait();
                Checker::unify(&ast, &m, &Value::Succ(Box::new(n.clone())), 0, &turbine)
            });
            let right = scope.spawn(|| {
                barrier.wait();
                Checker::unify(&ast, &n, &Value::Succ(Box::new(m.clone())), 0, &turbine)
            });
            (
                left.join().expect("left worker"),
                right.join().expect("right worker"),
            )
        });
        assert_ne!(
            left, right,
            "exactly one acyclic constraint may be committed"
        );
    }
}

#[test]
#[cfg(feature = "std")]
fn invalid_meta_ids_do_not_allocate_or_recurse() {
    let ast = Ast::new();
    let turbine = TurbineEngine::new(0);
    assert!(!turbine.solve_meta(MetaId(usize::MAX), Value::Bool));
    assert!(!Checker::unify(
        &ast,
        &Value::Meta(MetaId(usize::MAX), vec![]),
        &Value::Bool,
        0,
        &turbine
    ));
    assert_eq!(turbine.new_meta(), MetaId(0));
}

#[test]
fn linear_induction_rejects_duplication_and_discarding_but_accepts_single_use() {
    for body in ["if ih then ih else ih", "True"] {
        let source = format!(
            "(fn :^1 x -> ind(fn n -> Bool, x, fn n -> fn ih -> {body}, S(Z))) : fn :^1 (x : Bool) -> Bool"
        );
        let (ast, root) = parse(&source);
        assert!(matches!(
            elaborator::synthesize(&ast, root, &[]),
            Err(elaborator::Error::UsageMismatch {
                declared: Quantity::One,
                observed: Quantity::Omega,
                ..
            })
        ));
    }
    let (ast, root) = parse(
        "(fn :^1 x -> ind(fn n -> Bool, x, fn n -> fn :^1 ih -> ih, S(Z))) : fn :^1 (x : Bool) -> Bool",
    );
    assert!(elaborator::synthesize(&ast, root, &[]).is_ok());
}

#[test]
fn zero_quantity_arguments_are_erased_through_function_parameters() {
    let term = checked_erasure(
        "((fn f -> f Z) : fn (f : fn :^0 (n : Nat) -> Bool) -> Bool) (fn :^0 n -> True)",
    );
    assert_eq!(erasure::eval_runtime(&term, &[]), RuntimeTerm::Bool(true));
}

#[test]
fn erased_proof_domains_remove_their_lambda_binders() {
    let term = checked_erasure("((fn p -> True) : fn (p : Id(Nat, Z, Z)) -> Bool) refl(Z)");
    assert_eq!(erasure::eval_runtime(&term, &[]), RuntimeTerm::Bool(true));
}

#[test]
fn erased_type_domains_reindex_retained_arguments() {
    let term = checked_erasure(
        "((fn A -> fn :^1 x -> x) : fn (A : type) -> fn :^1 (x : A) -> A) Nat S(Z)",
    );
    assert_eq!(erasure::eval_runtime(&term, &[]), RuntimeTerm::Nat(1));
}

#[test]
fn erasure_preserves_both_projections_and_computational_unit() {
    for (field, expected) in [(1, RuntimeTerm::Nat(0)), (2, RuntimeTerm::Bool(true))] {
        let result_type = if field == 1 { "Nat" } else { "Bool" };
        let term = checked_erasure(&format!(
            "((fn p -> p.{field}) : fn (p : sigma (x : Nat) * Bool) -> {result_type}) ((Z, True) : sigma (x : Nat) * Bool)"
        ));
        assert_eq!(erasure::eval_runtime(&term, &[]), expected);
    }
    let term = checked_erasure("((fn :^1 u -> u) : fn :^1 (u : UnitType) -> UnitType) ()");
    assert_eq!(erasure::eval_runtime(&term, &[]), RuntimeTerm::Unit);
}

#[test]
fn erasure_keeps_recursion_step_arity_with_an_erased_index() {
    let term = checked_erasure("ind(fn n -> Nat, Z, fn :^0 n -> fn :^1 ih -> S(ih), S(S(Z)))");
    assert_eq!(erasure::eval_runtime(&term, &[]), RuntimeTerm::Nat(2));
}

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct NativeBuild(PathBuf);

impl Drop for NativeBuild {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn compile_and_run(term: &RuntimeTerm) -> u64 {
    let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    let directory = NativeBuild(
        std::env::temp_dir().join(format!("axiom0-v1-c99-{}-{id}", std::process::id())),
    );
    std::fs::create_dir(&directory.0).expect("unique native build directory");
    let source = directory.0.join("regression.c");
    let executable = directory.0.join(if cfg!(windows) {
        "regression.exe"
    } else {
        "regression"
    });
    let mut code = erasure::emit_c_code(term, "regression");
    code.push_str(
        "#include <stdio.h>\nint main(void) { printf(\"%llu\\n\", (unsigned long long)regression()); return 0; }\n"
    );
    std::fs::write(&source, &code).expect("write generated C");
    let compiler = std::env::var_os("CC").unwrap_or_else(|| "cc".into());
    let compiled = Process::new(compiler)
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("a native C99 compiler (CC or cc) is required for V1 validation");
    assert!(
        compiled.status.success(),
        "{}\n{code}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let result = Process::new(&executable)
        .output()
        .expect("execute generated C");
    assert!(result.status.success());
    String::from_utf8(result.stdout)
        .expect("numeric output")
        .trim()
        .parse()
        .expect("u64 result")
}

#[test]
fn c99_recursion_matches_runtime_including_nested_captures() {
    for (source, expected) in [
        ("ind(fn n -> Nat, Z, fn n -> fn :^1 ih -> S(ih), S(Z))", 1),
        ("ind(fn n -> Nat, S(Z), fn n -> fn :^1 ih -> S(ih), Z)", 1),
        (
            "ind(fn n -> Nat, Z, fn n -> fn ih -> ind(fn k -> Nat, ih, fn k -> fn :^1 acc -> S(acc), S(Z)), S(S(Z)))",
            2,
        ),
    ] {
        let term = checked_erasure(source);
        assert_eq!(
            erasure::eval_runtime(&term, &[]),
            RuntimeTerm::Nat(expected)
        );
        assert_eq!(compile_and_run(&term), expected);
    }
}

#[test]
fn c99_projects_second_components_and_preserves_saturating_successor() {
    let pair = RuntimeTerm::Pair(Box::new(RuntimeTerm::Nat(3)), Box::new(RuntimeTerm::Nat(7)));
    assert_eq!(compile_and_run(&RuntimeTerm::Snd(Box::new(pair))), 7);
    assert_eq!(
        compile_and_run(&RuntimeTerm::Succ(Box::new(RuntimeTerm::Nat(u64::MAX)))),
        u64::MAX
    );
}

#[test]
fn unsupported_c_terms_report_errors_instead_of_silently_returning_zero() {
    for term in [
        RuntimeTerm::Lam(Box::new(RuntimeTerm::Var(0))),
        RuntimeTerm::Closure(Box::new(RuntimeTerm::Var(0)), vec![]),
        RuntimeTerm::Pair(Box::new(RuntimeTerm::Nat(1)), Box::new(RuntimeTerm::Nat(2))),
        RuntimeTerm::App(Box::new(RuntimeTerm::Var(0)), Box::new(RuntimeTerm::Nat(1))),
    ] {
        assert!(matches!(
            erasure::try_emit_c_code(&term, "regression"),
            Err(CodegenError::UnsupportedTerm(_))
        ));
        assert!(erasure::emit_c_code(&term, "regression").starts_with("#error "));
    }
    assert_eq!(
        erasure::try_emit_c_code(&RuntimeTerm::Var(0), "regression"),
        Err(CodegenError::UnboundVariable(0))
    );
}

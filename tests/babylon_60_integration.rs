//! BABYLON-60 Ring-0 Integration Suite:
//! Verifies KUDURRU-64 cache-line alignment, TAMKARUM-60 linear budget gate conservation
//! (no double-spend, no drop), MUSHUSHU-0 dependent proof certification, and ENKI-60 erasure.

use micro_axiom_0::ast::{Ast, Command, Quantity};
use micro_axiom_0::elaborator::{Error, check_in_env, synthesize_in_env};
use micro_axiom_0::erasure::{RuntimeTerm, emit_c_code, erase_expression, eval_runtime};
use micro_axiom_0::eval::{Value, eval};
use micro_axiom_0::lexer::Lexer;
use micro_axiom_0::parser::Parser;
use micro_axiom_0::seqlock::SeqlockCell;
use micro_axiom_0::turbine::TurbineEngine;

fn exec_let(
    ast: &mut Ast,
    src: &str,
    names: &mut Vec<String>,
    env: &mut Vec<Value>,
    types: &mut Vec<Value>,
) {
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize_all().expect("lexing failed");
    let mut parser = Parser::new(&tokens, ast, names);
    let cmd = parser.parse_command().expect("parsing command failed");
    match cmd {
        Command::Let { name, ty, term } => {
            let term_ty = if let Some(ann) = ty {
                let ann_elab = synthesize_in_env(ast, ann, env, types).expect("type synthesis failed");
                assert!(matches!(ann_elab.ty, Value::Universe(_)));
                let expected = eval(ast, ann, env, None);
                check_in_env(ast, term, expected.clone(), env, types).expect("term check failed");
                expected
            } else {
                synthesize_in_env(ast, term, env, types)
                    .expect("term synthesis failed")
                    .ty
            };
            let val = eval(ast, term, env, None);
            names.push(name);
            env.push(val);
            types.push(term_ty);
        }
        Command::Eval(_) => panic!("expected Let/Theorem command"),
    }
}

#[test]
fn test_kudurru_64_cache_line_alignment_and_atomic_budget_telemetry() {
    // 1. Hardware cache-line alignment invariant (64 bytes minimum)
    assert_eq!(
        std::mem::align_of::<SeqlockCell<u64, 8>>(),
        64,
        "KUDURRU-64 SeqlockCell must be 64-byte aligned to prevent false sharing"
    );
    assert!(
        std::mem::size_of::<SeqlockCell<u64, 8>>() >= 64,
        "KUDURRU-64 8-word cell must span at least one 64B cache line"
    );

    // 2. Atomic snapshot publication for an 8-word KUDURRU-64 frame
    let kudurru = SeqlockCell::<u64, 8>::new([60, 120, 3600, 1, 0, 0, 0, 21_000]);
    let version = kudurru
        .try_write([59, 120, 3600, 1, 1, 0, 0, 21_000])
        .unwrap();
    assert_eq!(version, 2);

    let snap = kudurru.try_read(4).unwrap();
    assert_eq!(snap.version, 2);
    assert_eq!(snap.value[0], 59);
    assert_eq!(snap.value[7], 21_000);
}

#[test]
fn test_tamkarum_60_linear_budget_gate_rejects_double_spend_and_silent_drop() {
    let mut ast = Ast::new();
    let mut names = Vec::new();
    let mut env = Vec::new();
    let mut types = Vec::new();

    // Define linear KudurruBudgetToken with dependent validity proof (Q=0)
    exec_let(
        &mut ast,
        "module Babylon.Tamkarum; let BudgetToken : type 0 = sigma :^1 (sats : Nat) * Id(Nat, sats, sats);",
        &mut names,
        &mut env,
        &mut types,
    );

    // 1. Valid linear settlement: consumes the :^1 BudgetToken exactly once and returns settled sats
    exec_let(
        &mut ast,
        "theorem settle_rtb_bid : fn :^1 (tok : BudgetToken) -> Nat := fn :^1 tok -> tok.1;",
        &mut names,
        &mut env,
        &mut types,
    );

    // 2. Double-spend attempt: using linear token twice in a pair must fail with UsageMismatch(One, Omega)
    let double_spend_ty = {
        let mut l = Lexer::new("fn :^1 (tok : BudgetToken) -> sigma (a : Nat) * Nat");
        let t = l.tokenize_all().unwrap();
        let mut p = Parser::new(&t, &mut ast, &names);
        let e = p.parse_expression().unwrap();
        eval(&ast, e, &env, None)
    };
    let double_spend_term = {
        let mut l = Lexer::new("fn :^1 tok -> (tok.1, tok.1)");
        let t = l.tokenize_all().unwrap();
        let mut p = Parser::new(&t, &mut ast, &names);
        p.parse_expression().unwrap()
    };
    assert_eq!(
        check_in_env(&ast, double_spend_term, double_spend_ty, &env, &types).unwrap_err(),
        Error::UsageMismatch {
            expr: double_spend_term,
            declared: Quantity::One,
            observed: Quantity::Omega,
        }
    );

    // 3. Silent drop attempt: ignoring linear token must fail with UsageMismatch(One, Zero)
    let drop_ty = {
        let mut l = Lexer::new("fn :^1 (tok : BudgetToken) -> Nat");
        let t = l.tokenize_all().unwrap();
        let mut p = Parser::new(&t, &mut ast, &names);
        let e = p.parse_expression().unwrap();
        eval(&ast, e, &env, None)
    };
    let drop_term = {
        let mut l = Lexer::new("fn :^1 tok -> Z");
        let t = l.tokenize_all().unwrap();
        let mut p = Parser::new(&t, &mut ast, &names);
        p.parse_expression().unwrap()
    };
    assert_eq!(
        check_in_env(&ast, drop_term, drop_ty, &env, &types).unwrap_err(),
        Error::UsageMismatch {
            expr: drop_term,
            declared: Quantity::One,
            observed: Quantity::Zero,
        }
    );
}

#[test]
fn test_enki_60_scitt_receipt_proof_erasure_and_c99_transduction() {
    let mut ast = Ast::new();
    // Certified SCITT receipt: pair of payload (3 sats) and Martin-Löf equality proof,
    // passed through a polymorphic proof-erased gate (:^0) and linear consumer (:^1).
    let src = "(fn :^0 P -> fn :^1 receipt -> receipt.1) (Id(Nat, S(S(S(Z))), S(S(S(Z))))) (S(S(S(Z))), refl(S(S(S(Z)))))";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize_all().unwrap();
    let mut parser = Parser::new(&tokens, &mut ast, &[]);
    let root = parser.parse_expression().unwrap();

    // Erase all Q=0 types and refl proofs
    let erased = erase_expression(&ast, root).unwrap();
    let result = eval_runtime(&erased, &[]);
    assert_eq!(result, RuntimeTerm::Nat(3));

    let c99 = emit_c_code(&result, "babylon_enki60_settle");
    assert!(c99.contains("uint64_t babylon_enki60_settle()"));
    assert!(c99.contains("return 3ULL;"));
}

#[test]
fn test_larsa_120_parallel_batch_certification() {
    let mut ast = Ast::new();
    let mut roots = Vec::new();
    for _ in 0..60 {
        let mut lexer = Lexer::new("fn :^1 x -> x");
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(&tokens, &mut ast, &[]);
        roots.push(parser.parse_expression().unwrap());
    }

    let expected = {
        let mut lexer = Lexer::new("fn :^1 (x : Nat) -> Nat");
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(&tokens, &mut ast, &[]);
        let ty_expr = parser.parse_expression().unwrap();
        eval(&ast, ty_expr, &[], None)
    };

    let turbine = TurbineEngine::for_ast(&ast);
    let results = turbine.check_batch_parallel(&ast, &roots, expected, &[]);
    assert_eq!(results.len(), 60);
    assert!(results.iter().all(|r| r.is_ok()));
}

//! Test suite for QTT Proof Erasure, Runtime Unboxing, and C-ABI Codegen.

use micro_axiom_0::ast::Ast;
use micro_axiom_0::elaborator::check;
use micro_axiom_0::erasure::{emit_c_code, erase_expression, eval_runtime, RuntimeTerm};
use micro_axiom_0::eval::eval;
use micro_axiom_0::lexer::Lexer;
use micro_axiom_0::parser::Parser;

fn parse(ast: &mut Ast, src: &str, names: &[String]) -> micro_axiom_0::ast::ExprId {
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize_all().expect("lexing failed");
    let mut parser = Parser::new(&tokens, ast, names);
    parser.parse_expression().expect("parsing failed")
}

#[test]
fn test_erasure_peano_unboxing() {
    let mut ast = Ast::new();
    let term = parse(&mut ast, "S(S(S(Z)))", &[]);
    let ty = parse(&mut ast, "Nat", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());

    let erased = erase_expression(&ast, term).unwrap();
    assert_eq!(erased, RuntimeTerm::Nat(3));
    assert_eq!(erased.node_count(), 1, "Unboxed Nat must be a single scalar node");
}

#[test]
fn test_erasure_martin_lof_j_eliminator_collapses_to_base() {
    let mut ast = Ast::new();
    // J(P, d, a, refl(a)) where d is S(S(Z)) and proof is refl(Z)
    let term = parse(&mut ast, "J(fn b -> fn _ -> Nat, S(S(Z)), refl(Z))", &[]);
    let ty = parse(&mut ast, "Nat", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());

    let erased = erase_expression(&ast, term).unwrap();
    // J is completely eradicated; runtime term is directly the unboxed base value 2!
    assert_eq!(erased, RuntimeTerm::Nat(2));
}

#[test]
fn test_erasure_sigma_subset_refinement_unboxing() {
    let mut ast = Ast::new();
    // Pair of a value and its equality proof: (S(Z), refl(S(Z)))
    // Type: sigma (n : Nat) * Id(Nat, n, n)
    let term = parse(&mut ast, "(S(Z), refl(S(Z)))", &[]);
    let ty = parse(&mut ast, "sigma (n : Nat) * Id(Nat, n, n)", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());

    let erased = erase_expression(&ast, term).unwrap();
    // The proof component Id/refl is erased; the pair unboxes to just the scalar 1!
    assert_eq!(erased, RuntimeTerm::Nat(1));
}

#[test]
fn test_erasure_fst_projection_on_unboxed_pair() {
    let mut ast = Ast::new();
    let term = parse(&mut ast, "(S(S(Z)), refl(S(S(Z)))).1", &[]);
    let erased = erase_expression(&ast, term).unwrap();
    assert_eq!(erased, RuntimeTerm::Nat(2));
}

#[test]
fn test_erasure_zero_quantity_function_binder_eliminated() {
    let mut ast = Ast::new();
    // A function taking a proof (Q=0) and returning Nat: fn :^0 (p: Id(Nat, Z, Z)) -> S(S(S(Z)))
    let term = parse(&mut ast, "fn :^0 p -> S(S(S(Z)))", &[]);
    let ty = parse(&mut ast, "fn :^0 (p: Id(Nat, Z, Z)) -> Nat", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());

    let erased = erase_expression(&ast, term).unwrap();
    // Lambda binder with Q=0 is eliminated; term directly becomes the unboxed body 3!
    assert_eq!(erased, RuntimeTerm::Nat(3));
}

#[test]
fn test_erasure_if_conditional_branching() {
    let mut ast = Ast::new();
    let term = parse(&mut ast, "if True then S(Z) else Z", &[]);
    let ty = parse(&mut ast, "Nat", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());

    let erased = erase_expression(&ast, term).unwrap();
    assert_eq!(
        erased,
        RuntimeTerm::If {
            cond: Box::new(RuntimeTerm::Bool(true)),
            conseq: Box::new(RuntimeTerm::Nat(1)),
            alt: Box::new(RuntimeTerm::Nat(0)),
        }
    );

    let res = eval_runtime(&erased, &[]);
    assert_eq!(res, RuntimeTerm::Nat(1));
}

#[test]
fn test_erasure_peano_recursion_evaluation() {
    let mut ast = Ast::new();
    // ind(fn _ -> Nat, S(Z), fn _ -> fn ih -> S(ih), S(S(Z))) => computes 1 + 2 = 3
    let term = parse(&mut ast, "ind(fn _ -> Nat, S(Z), fn _ -> fn ih -> S(ih), S(S(Z)))", &[]);
    let ty = parse(&mut ast, "Nat", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());

    let erased = erase_expression(&ast, term).unwrap();
    let res = eval_runtime(&erased, &[]);
    assert_eq!(res, RuntimeTerm::Nat(3));
}

#[test]
fn test_erasure_c_code_emission() {
    let mut ast = Ast::new();
    let term = parse(&mut ast, "if True then S(S(Z)) else Z", &[]);
    let erased = erase_expression(&ast, term).unwrap();
    let c_code = emit_c_code(&erased, "sovereign_kernel_calc");

    assert!(c_code.contains("uint64_t sovereign_kernel_calc()"));
    assert!(c_code.contains("true"));
    assert!(c_code.contains("2ULL"));
    assert!(c_code.contains("0ULL"));
}

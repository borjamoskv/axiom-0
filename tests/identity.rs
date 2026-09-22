use micro_axiom_0::ast::Ast;
use micro_axiom_0::elaborator;
use micro_axiom_0::eval::{Value, eval};
use micro_axiom_0::lexer::Lexer;
use micro_axiom_0::parser::Parser;

#[test]
fn test_identity_refl_canonical() {
    let src = "refl(Z)";
    let expected_ty_src = "Id(Nat, Z, Z)";

    let mut ast = Ast::new();
    let term = {
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(&tokens, &mut ast, &[]);
        parser.parse_expression().unwrap()
    };

    let expected_ty_expr = {
        let mut lexer = Lexer::new(expected_ty_src);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(&tokens, &mut ast, &[]);
        parser.parse_expression().unwrap()
    };

    let expected_val = eval(&ast, expected_ty_expr, &[], None);
    let elab = elaborator::check(&ast, term, expected_val.clone(), &[])
        .expect("refl(Z) must check against Id(Nat, Z, Z)");
    assert_eq!(elab.ty, expected_val);
}

#[test]
fn test_identity_refl_mismatch_rejected() {
    let src = "refl(Z)";
    let expected_ty_src = "Id(Nat, Z, S(Z))";

    let mut ast = Ast::new();
    let term = {
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(&tokens, &mut ast, &[]);
        parser.parse_expression().unwrap()
    };

    let expected_ty_expr = {
        let mut lexer = Lexer::new(expected_ty_src);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(&tokens, &mut ast, &[]);
        parser.parse_expression().unwrap()
    };

    let expected_val = eval(&ast, expected_ty_expr, &[], None);
    let check_res = elaborator::check(&ast, term, expected_val, &[]);
    assert!(check_res.is_err(), "refl(Z) must be rejected against Id(Nat, Z, S(Z))");
}

#[test]
fn test_identity_j_definitional_computation() {
    // J(mot, base, refl(Z)) == base
    let src = "J(fn b -> fn p -> Nat, S(Z), refl(Z))";

    let mut ast = Ast::new();
    let term = {
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(&tokens, &mut ast, &[]);
        parser.parse_expression().unwrap()
    };

    let elab = elaborator::synthesize(&ast, term, &[])
        .expect("Synthesis for canonical J term failed");
    assert_eq!(elab.ty, Value::NatType);

    // Definitional reduction rule: J(P, d, refl) = d
    let val = eval(&ast, term, &[], None);
    assert_eq!(val, Value::Succ(Box::new(Value::Zero)));
}

#[test]
fn test_identity_symmetry_theorem() {
    // Proving Symmetry of Equality:
    // forall (x: Nat) (y: Nat) (p: Id(Nat, x, y)) -> Id(Nat, y, x)
    // sym = fn x -> fn y -> fn p -> J(fn b -> fn _ -> Id(Nat, b, x), refl(x), p)
    let src = "fn x -> fn y -> fn p -> J(fn b -> fn p2 -> Id(Nat, b, x), refl(x), p)";
    let ty_src = "fn (x: Nat) -> fn (y: Nat) -> fn (p: Id(Nat, x, y)) -> Id(Nat, y, x)";

    let mut ast = Ast::new();
    let term = {
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(&tokens, &mut ast, &[]);
        parser.parse_expression().unwrap()
    };

    let ty_expr = {
        let mut lexer = Lexer::new(ty_src);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(&tokens, &mut ast, &[]);
        parser.parse_expression().unwrap()
    };

    let expected_val = eval(&ast, ty_expr, &[], None);
    let elab = elaborator::check(&ast, term, expected_val, &[])
        .expect("Type checking symmetry theorem failed");
    assert!(elab.usages.is_empty());
}

#[test]
fn test_identity_qtt_proof_erasure() {
    // Theorem using QTT proof erasure :^0:
    // fn :^0 (p: Id(Nat, Z, Z)) -> Nat
    let src = "fn :^0 p -> S(Z)";
    let ty_src = "fn :^0 (p: Id(Nat, Z, Z)) -> Nat";

    let mut ast = Ast::new();
    let term = {
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(&tokens, &mut ast, &[]);
        parser.parse_expression().unwrap()
    };

    let ty_expr = {
        let mut lexer = Lexer::new(ty_src);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(&tokens, &mut ast, &[]);
        parser.parse_expression().unwrap()
    };

    let expected_val = eval(&ast, ty_expr, &[], None);
    let elab = elaborator::check(&ast, term, expected_val, &[])
        .expect("Type check with erased proof parameter failed");
    assert!(elab.usages.is_empty(), "Erased proof must consume 0 resources at runtime");
}

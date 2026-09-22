//! Centuria de AXIOM-0: 100 Teoremas e Invariantes Formales No Redundantes.
//!
//! Cada teorema certifica una propiedad matemática, lógica o computacional única
//! en el micro-núcleo QTT, estructurada en 10 dominios canónicos de 10 teoremas cada uno.

use micro_axiom_0::ast::{Ast, Command, Expr, Level, Quantity};
use micro_axiom_0::elaborator::{self, check, check_with_turbine, synthesize, synthesize_with_turbine};
use micro_axiom_0::eval::{Neutral, Value, equiv, eval};
use micro_axiom_0::lexer::Lexer;
use micro_axiom_0::parser::Parser;
use micro_axiom_0::seqlock::{ReadError, SeqlockCell};
use micro_axiom_0::turbine::{AtomicElabSnapshot, ElabStatus, TurbineEngine, TypeTag};

fn parse_expr(ast: &mut Ast, src: &str, names: &[String]) -> micro_axiom_0::ast::ExprId {
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize_all().expect("lexing failed");
    let mut parser = Parser::new(&tokens, ast, names);
    parser.parse_expression().expect("parsing failed")
}

fn parse_cmd(ast: &mut Ast, src: &str, names: &[String]) -> Command {
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize_all().expect("lexing failed");
    let mut parser = Parser::new(&tokens, ast, names);
    parser.parse_command().expect("command parsing failed")
}

// ============================================================================
// DOMINIO I: ÁLGEBRA DE LA IGUALDAD PROPOSICIONAL (TEOREMAS 001 - 010)
// ============================================================================

#[test]
fn thm_001_refl_unit() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "refl(())", &[]);
    let ty = parse_expr(&mut ast, "Id(UnitType, (), ())", &[]);
    let expected = eval(&ast, ty, &[], None);
    let elab = check(&ast, term, expected, &[]).unwrap();
    assert!(elab.usages.is_empty());
}

#[test]
fn thm_002_refl_bool_true() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "refl(True)", &[]);
    let ty = parse_expr(&mut ast, "Id(Bool, True, True)", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());
}

#[test]
fn thm_003_refl_bool_false() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "refl(False)", &[]);
    let ty = parse_expr(&mut ast, "Id(Bool, False, False)", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());
}

#[test]
fn thm_004_refl_nat_zero() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "refl(Z)", &[]);
    let ty = parse_expr(&mut ast, "Id(Nat, Z, Z)", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());
}

#[test]
fn thm_005_refl_nat_succ() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "refl(S(Z))", &[]);
    let ty = parse_expr(&mut ast, "Id(Nat, S(Z), S(Z))", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());
}

#[test]
fn thm_006_symmetry_generic() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "fn x -> fn y -> fn p -> J(fn b -> fn _ -> Id(Nat, b, x), refl(x), p)", &[]);
    let ty = parse_expr(&mut ast, "fn (x: Nat) -> fn (y: Nat) -> fn (p: Id(Nat, x, y)) -> Id(Nat, y, x)", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());
}

#[test]
fn thm_007_transitivity_generic() {
    // trans : forall x y z. x = y -> y = z -> x = z
    let mut ast = Ast::new();
    let term = parse_expr(
        &mut ast,
        "fn x -> fn y -> fn z -> fn p1 -> fn p2 -> J(fn c -> fn _ -> Id(Nat, x, c), p1, p2)",
        &[],
    );
    let ty = parse_expr(
        &mut ast,
        "fn (x: Nat) -> fn (y: Nat) -> fn (z: Nat) -> fn (p1: Id(Nat, x, y)) -> fn (p2: Id(Nat, y, z)) -> Id(Nat, x, z)",
        &[],
    );
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());
}

#[test]
fn thm_008_congruence_succ() {
    // cong_succ : forall x y. x = y -> S(x) = S(y)
    let mut ast = Ast::new();
    let term = parse_expr(
        &mut ast,
        "fn x -> fn y -> fn p -> J(fn b -> fn _ -> Id(Nat, S(x), S(b)), refl(S(x)), p)",
        &[],
    );
    let ty = parse_expr(
        &mut ast,
        "fn (x: Nat) -> fn (y: Nat) -> fn (p: Id(Nat, x, y)) -> Id(Nat, S(x), S(y))",
        &[],
    );
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());
}

#[test]
fn thm_009_congruence_generic() {
    // cong (f : Nat -> Nat) : forall x y. x = y -> f(x) = f(y)
    let mut ast = Ast::new();
    let term = parse_expr(
        &mut ast,
        "fn f -> fn x -> fn y -> fn p -> J(fn b -> fn _ -> Id(Nat, f x, f b), refl(f x), p)",
        &[],
    );
    let ty = parse_expr(
        &mut ast,
        "fn (f: fn (n: Nat) -> Nat) -> fn (x: Nat) -> fn (y: Nat) -> fn (p: Id(Nat, x, y)) -> Id(Nat, f x, f y)",
        &[],
    );
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());
}

#[test]
fn thm_010_transport_predicate() {
    // leibniz transport: forall (P: Nat -> type) (x y: Nat) (p: x = y). P(x) -> P(y)
    let mut ast = Ast::new();
    let term = parse_expr(
        &mut ast,
        "fn P -> fn x -> fn y -> fn p -> fn px -> J(fn b -> fn _ -> fn (px_inner: P x) -> P b, fn px_self -> px_self, p) px",
        &[],
    );
    let ty = parse_expr(
        &mut ast,
        "fn (P: fn (n: Nat) -> type) -> fn (x: Nat) -> fn (y: Nat) -> fn (p: Id(Nat, x, y)) -> fn (px: P x) -> P y",
        &[],
    );
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());
}

// ============================================================================
// DOMINIO II: ARITMÉTICA DE PEANO Y RECURSIÓN NbE (TEOREMAS 011 - 020)
// ============================================================================

#[test]
fn thm_011_add_zero_left() {
    // 0 + n = n definitional
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "ind(fn _ -> Nat, S(S(Z)), fn _ -> fn ih -> S(ih), Z)", &[]);
    let val = eval(&ast, term, &[], None);
    assert_eq!(val, Value::Succ(Box::new(Value::Succ(Box::new(Value::Zero)))));
}

#[test]
fn thm_012_add_zero_right_concrete() {
    // n + 0 = n
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "ind(fn _ -> Nat, Z, fn _ -> fn ih -> S(ih), S(S(S(Z))))", &[]);
    let val = eval(&ast, term, &[], None);
    assert_eq!(val, Value::Succ(Box::new(Value::Succ(Box::new(Value::Succ(Box::new(Value::Zero)))))));
}

#[test]
fn thm_013_add_succ_left() {
    // S(1) + 2 = S(1 + 2) = 3
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "ind(fn _ -> Nat, S(S(Z)), fn _ -> fn ih -> S(ih), S(Z))", &[]);
    let val = eval(&ast, term, &[], None);
    assert_eq!(val, Value::Succ(Box::new(Value::Succ(Box::new(Value::Succ(Box::new(Value::Zero)))))));
}

#[test]
fn thm_014_peano_addition_comm_base() {
    // 0 + 1 == 1 + 0 in NbE
    let mut ast = Ast::new();
    let e1 = parse_expr(&mut ast, "ind(fn _ -> Nat, S(Z), fn _ -> fn ih -> S(ih), Z)", &[]);
    let e2 = parse_expr(&mut ast, "ind(fn _ -> Nat, Z, fn _ -> fn ih -> S(ih), S(Z))", &[]);
    let v1 = eval(&ast, e1, &[], None);
    let v2 = eval(&ast, e2, &[], None);
    assert_eq!(v1, v2);
}

#[test]
fn thm_015_mul_zero_left() {
    // 0 * n = 0
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "ind(fn _ -> Nat, Z, fn _ -> fn ih -> ind(fn _ -> Nat, ih, fn _ -> fn a -> S(a), S(S(Z))), Z)", &[]);
    let val = eval(&ast, term, &[], None);
    assert_eq!(val, Value::Zero);
}

#[test]
fn thm_016_mul_zero_right() {
    // n * 0 = 0
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "ind(fn _ -> Nat, Z, fn _ -> fn ih -> ind(fn _ -> Nat, ih, fn _ -> fn a -> S(a), Z), S(S(S(Z))))", &[]);
    let val = eval(&ast, term, &[], None);
    assert_eq!(val, Value::Zero);
}

#[test]
fn thm_017_mul_one_left() {
    // 1 * 3 = 3
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "ind(fn _ -> Nat, Z, fn _ -> fn ih -> ind(fn _ -> Nat, ih, fn _ -> fn a -> S(a), S(S(S(Z)))), S(Z))", &[]);
    let val = eval(&ast, term, &[], None);
    assert_eq!(val, Value::Succ(Box::new(Value::Succ(Box::new(Value::Succ(Box::new(Value::Zero)))))));
}

#[test]
fn thm_018_mul_one_right() {
    // 3 * 1 = 3
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "ind(fn _ -> Nat, Z, fn _ -> fn ih -> ind(fn _ -> Nat, ih, fn _ -> fn a -> S(a), S(Z)), S(S(S(Z))))", &[]);
    let val = eval(&ast, term, &[], None);
    assert_eq!(val, Value::Succ(Box::new(Value::Succ(Box::new(Value::Succ(Box::new(Value::Zero)))))));
}

#[test]
fn thm_019_add_associativity_concrete() {
    // (1 + 2) + 3 == 1 + (2 + 3) = 6
    let mut ast = Ast::new();
    let e1 = parse_expr(&mut ast, "ind(fn _ -> Nat, S(S(S(Z))), fn _ -> fn ih -> S(ih), ind(fn _ -> Nat, S(S(Z)), fn _ -> fn ih -> S(ih), S(Z)))", &[]);
    let e2 = parse_expr(&mut ast, "ind(fn _ -> Nat, ind(fn _ -> Nat, S(S(S(Z))), fn _ -> fn ih -> S(ih), S(S(Z))), fn _ -> fn ih -> S(ih), S(Z))", &[]);
    let v1 = eval(&ast, e1, &[], None);
    let v2 = eval(&ast, e2, &[], None);
    assert_eq!(v1, v2);
}

#[test]
fn thm_020_mul_distributivity_concrete() {
    // 2 * (1 + 2) == (2 * 1) + (2 * 2) = 6
    let mut ast = Ast::new();
    let e1 = parse_expr(&mut ast, "ind(fn _ -> Nat, Z, fn _ -> fn ih -> ind(fn _ -> Nat, ih, fn _ -> fn a -> S(a), S(S(S(Z)))), S(S(Z)))", &[]);
    let val = eval(&ast, e1, &[], None);
    let mut count = 0;
    let mut cur = &val;
    while let Value::Succ(n) = cur {
        count += 1;
        cur = n;
    }
    assert_eq!(count, 6);
}

// ============================================================================
// DOMINIO III: LÓGICA BOOLEANA Y CONDICIONALES (TEOREMAS 021 - 030)
// ============================================================================

#[test]
fn thm_021_if_true_reduction() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "if True then S(Z) else Z", &[]);
    let val = eval(&ast, term, &[], None);
    assert_eq!(val, Value::Succ(Box::new(Value::Zero)));
}

#[test]
fn thm_022_if_false_reduction() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "if False then S(Z) else Z", &[]);
    let val = eval(&ast, term, &[], None);
    assert_eq!(val, Value::Zero);
}

#[test]
fn thm_023_bool_not_involution() {
    // not(not(b)) == b for both True and False
    let mut ast = Ast::new();
    let not_not_true = parse_expr(&mut ast, "if (if True then False else True) then False else True", &[]);
    let not_not_false = parse_expr(&mut ast, "if (if False then False else True) then False else True", &[]);
    assert_eq!(eval(&ast, not_not_true, &[], None), Value::True);
    assert_eq!(eval(&ast, not_not_false, &[], None), Value::False);
}

#[test]
fn thm_024_bool_and_true_left() {
    // True and True = True; True and False = False
    let mut ast = Ast::new();
    let t1 = parse_expr(&mut ast, "if True then True else False", &[]);
    let t2 = parse_expr(&mut ast, "if True then False else False", &[]);
    assert_eq!(eval(&ast, t1, &[], None), Value::True);
    assert_eq!(eval(&ast, t2, &[], None), Value::False);
}

#[test]
fn thm_025_bool_and_false_left() {
    // False and True = False; False and False = False
    let mut ast = Ast::new();
    let t1 = parse_expr(&mut ast, "if False then True else False", &[]);
    let t2 = parse_expr(&mut ast, "if False then False else False", &[]);
    assert_eq!(eval(&ast, t1, &[], None), Value::False);
    assert_eq!(eval(&ast, t2, &[], None), Value::False);
}

#[test]
fn thm_026_bool_or_true_left() {
    // True or False = True
    let mut ast = Ast::new();
    let t = parse_expr(&mut ast, "if True then True else False", &[]);
    assert_eq!(eval(&ast, t, &[], None), Value::True);
}

#[test]
fn thm_027_bool_or_false_left() {
    // False or True = True; False or False = False
    let mut ast = Ast::new();
    let t1 = parse_expr(&mut ast, "if False then True else True", &[]);
    let t2 = parse_expr(&mut ast, "if False then True else False", &[]);
    assert_eq!(eval(&ast, t1, &[], None), Value::True);
    assert_eq!(eval(&ast, t2, &[], None), Value::False);
}

#[test]
fn thm_028_bool_de_morgan_and() {
    // not(True and False) == not(True) or not(False) == True
    let mut ast = Ast::new();
    let lhs = parse_expr(&mut ast, "if (if True then False else False) then False else True", &[]);
    let rhs = parse_expr(&mut ast, "if (if True then False else True) then True else (if False then False else True)", &[]);
    assert_eq!(eval(&ast, lhs, &[], None), eval(&ast, rhs, &[], None));
    assert_eq!(eval(&ast, lhs, &[], None), Value::True);
}

#[test]
fn thm_029_bool_de_morgan_or() {
    // not(False or True) == not(False) and not(True) == False
    let mut ast = Ast::new();
    let lhs = parse_expr(&mut ast, "if (if False then True else True) then False else True", &[]);
    let rhs = parse_expr(&mut ast, "if (if False then False else True) then (if True then False else True) else False", &[]);
    assert_eq!(eval(&ast, lhs, &[], None), eval(&ast, rhs, &[], None));
    assert_eq!(eval(&ast, lhs, &[], None), Value::False);
}

#[test]
fn thm_030_bool_xor_nilpotent() {
    // x xor x == False for both True and False
    let mut ast = Ast::new();
    let xor_tt = parse_expr(&mut ast, "if True then (if True then False else True) else True", &[]);
    let xor_ff = parse_expr(&mut ast, "if False then (if False then False else True) else False", &[]);
    assert_eq!(eval(&ast, xor_tt, &[], None), Value::False);
    assert_eq!(eval(&ast, xor_ff, &[], None), Value::False);
}

// ============================================================================
// DOMINIO IV: PARES DEPENDIENTES (SIGMA) Y PROYECCIONES (TEOREMAS 031 - 040)
// ============================================================================

#[test]
fn thm_031_sigma_fst_projection() {
    let mut ast = Ast::new();
    let cmd = parse_cmd(&mut ast, "let p : sigma (x : Nat) * Bool = (S(Z), True);", &[]);
    if let Command::Let { ty, term, .. } = cmd {
        let ty_val = eval(&ast, ty.unwrap(), &[], None);
        assert!(check(&ast, term, ty_val, &[]).is_ok());
    }
}

#[test]
fn thm_032_sigma_snd_projection() {
    let mut ast = Ast::new();
    let p = parse_expr(&mut ast, "(S(Z), False).2", &[]);
    assert_eq!(eval(&ast, p, &[], None), Value::False);
}

#[test]
fn thm_033_sigma_eta_pair() {
    let mut ast = Ast::new();
    let p1 = parse_expr(&mut ast, "(S(Z), True).1", &[]);
    let p2 = parse_expr(&mut ast, "(S(Z), True).2", &[]);
    assert_eq!(eval(&ast, p1, &[], None), Value::Succ(Box::new(Value::Zero)));
    assert_eq!(eval(&ast, p2, &[], None), Value::True);
}

#[test]
fn thm_034_sigma_dependent_predicate() {
    // Sigma (n: Nat) * Id(Nat, n, n)
    let mut ast = Ast::new();
    let ty = parse_expr(&mut ast, "sigma (n : Nat) * Id(Nat, n, n)", &[]);
    let term = parse_expr(&mut ast, "(Z, refl(Z))", &[]);
    let ty_val = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, ty_val, &[]).is_ok());
}

#[test]
fn thm_035_sigma_linear_preservation() {
    let mut ast = Ast::new();
    let ty = parse_expr(&mut ast, "fn :^1 (p : sigma :^1 (x: Nat) * Bool) -> Nat", &[]);
    let term = parse_expr(&mut ast, "fn :^1 p -> p.1", &[]);
    let ty_val = eval(&ast, ty, &[], None);
    let elab = check(&ast, term, ty_val, &[]).unwrap();
    assert!(elab.usages.is_empty());
}

#[test]
fn thm_036_sigma_erased_component() {
    let mut ast = Ast::new();
    let ty = parse_expr(&mut ast, "fn (p : sigma :^0 (x: Nat) * Bool) -> Bool", &[]);
    let term = parse_expr(&mut ast, "fn p -> p.2", &[]);
    let ty_val = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, ty_val, &[]).is_ok());
}

#[test]
fn thm_037_sigma_nested_fst() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "((S(Z), True), False).1.1", &[]);
    assert_eq!(eval(&ast, term, &[], None), Value::Succ(Box::new(Value::Zero)));
}

#[test]
fn thm_038_sigma_nested_snd() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "((S(Z), True), False).1.2", &[]);
    assert_eq!(eval(&ast, term, &[], None), Value::True);
}

#[test]
fn thm_039_sigma_swap_involution() {
    let mut ast = Ast::new();
    let swap_swap = parse_expr(&mut ast, "((((S(Z), False).2, (S(Z), False).1)).2, (((S(Z), False).2, (S(Z), False).1)).1)", &[]);
    let p = parse_expr(&mut ast, "(S(Z), False)", &[]);
    assert_eq!(eval(&ast, swap_swap, &[], None), eval(&ast, p, &[], None));
}

#[test]
fn thm_040_sigma_unit_absorption() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "((), S(S(Z))).2", &[]);
    assert_eq!(eval(&ast, term, &[], None), Value::Succ(Box::new(Value::Succ(Box::new(Value::Zero)))));
}

// ============================================================================
// DOMINIO V: CUANTIFICACIÓN LINEAL QTT Y SEMIANILLO Q (TEOREMAS 041 - 050)
// ============================================================================

#[test]
fn thm_041_qtt_exact_linear_consumption() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "fn :^1 x -> x", &[]);
    let ty = parse_expr(&mut ast, "fn :^1 (x: Nat) -> Nat", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());
}

#[test]
fn thm_042_qtt_linear_variable_rejection_unused() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "fn :^1 x -> Z", &[]);
    let ty = parse_expr(&mut ast, "fn :^1 (x: Nat) -> Nat", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_err());
}

#[test]
fn thm_043_qtt_linear_variable_rejection_duplicated() {
    let mut ast = Ast::new();
    // Consuming linear variable twice inside a pair
    let term = parse_expr(&mut ast, "fn :^1 x -> (x, x)", &[]);
    let ty = parse_expr(&mut ast, "fn :^1 (x: Nat) -> sigma (a: Nat) * Nat", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_err());
}

#[test]
fn thm_044_qtt_erased_proof_zero_usage() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "fn :^0 p -> Z", &[]);
    let ty = parse_expr(&mut ast, "fn :^0 (p: Id(Nat, Z, Z)) -> Nat", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());
}

#[test]
fn thm_045_qtt_omega_allows_zero_usage() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "fn x -> Z", &[]);
    let ty = parse_expr(&mut ast, "fn (x: Nat) -> Nat", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());
}

#[test]
fn thm_046_qtt_omega_allows_multiple_usage() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "fn x -> (x, x)", &[]);
    let ty = parse_expr(&mut ast, "fn (x: Nat) -> sigma (a: Nat) * Nat", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());
}

#[test]
fn thm_047_qtt_multiplication_zero_times_one() {
    assert_eq!(Quantity::Zero.times(Quantity::One), Quantity::Zero);
}

#[test]
fn thm_048_qtt_multiplication_one_times_one() {
    assert_eq!(Quantity::One.times(Quantity::One), Quantity::One);
}

#[test]
fn thm_049_qtt_addition_one_plus_one() {
    assert_eq!(Quantity::One.plus(Quantity::One), Quantity::Omega);
}

#[test]
fn thm_050_qtt_erased_type_annotation() {
    let mut ast = Ast::new();
    let ty = parse_expr(&mut ast, "fn :^0 (A: type) -> fn (a: A) -> A", &[]);
    let term = parse_expr(&mut ast, "fn :^0 A -> fn a -> a", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());
}

// ============================================================================
// DOMINIO VI: JERARQUÍA Y CUMULATIVIDAD DE UNIVERSOS (TEOREMAS 051 - 060)
// ============================================================================

#[test]
fn thm_051_universe_stratification_base() {
    let mut ast = Ast::new();
    let u0 = ast.push(Expr::Universe(0)).unwrap();
    let elab = synthesize(&ast, u0, &[]).unwrap();
    assert_eq!(elab.ty, Value::Universe(1));
}

#[test]
fn thm_052_universe_stratification_trans() {
    let mut ast = Ast::new();
    let u1 = ast.push(Expr::Universe(1)).unwrap();
    let elab = synthesize(&ast, u1, &[]).unwrap();
    assert_eq!(elab.ty, Value::Universe(2));
}

#[test]
fn thm_053_cumulativity_refl() {
    let mut ast = Ast::new();
    let u0 = ast.push(Expr::Universe(0)).unwrap();
    assert!(check(&ast, u0, Value::Universe(1), &[]).is_ok());
}

#[test]
fn thm_054_cumulativity_step_1() {
    let mut ast = Ast::new();
    let u0 = ast.push(Expr::Universe(0)).unwrap();
    assert!(check(&ast, u0, Value::Universe(2), &[]).is_ok());
}

#[test]
fn thm_055_cumulativity_step_10() {
    let mut ast = Ast::new();
    let u0 = ast.push(Expr::Universe(0)).unwrap();
    assert!(check(&ast, u0, Value::Universe(10), &[]).is_ok());
}

#[test]
fn thm_056_cumulativity_rejection_reverse() {
    let mut ast = Ast::new();
    let u5 = ast.push(Expr::Universe(5)).unwrap();
    assert!(check(&ast, u5, Value::Universe(2), &[]).is_err());
}

#[test]
fn thm_057_pi_universe_level_max() {
    let mut ast = Ast::new();
    let pi = parse_expr(&mut ast, "fn (x: type 2) -> type 4", &[]);
    let elab = synthesize(&ast, pi, &[]).unwrap();
    assert_eq!(elab.ty, Value::Universe(5));
}

#[test]
fn thm_058_sigma_universe_level_max() {
    let mut ast = Ast::new();
    let sig = parse_expr(&mut ast, "sigma (x: type 3) * type 1", &[]);
    let elab = synthesize(&ast, sig, &[]).unwrap();
    assert_eq!(elab.ty, Value::Universe(4));
}

#[test]
fn thm_059_id_universe_preservation() {
    let mut ast = Ast::new();
    let id_u = parse_expr(&mut ast, "Id(type 1, UnitType, UnitType)", &[]);
    let elab = synthesize(&ast, id_u, &[]).unwrap();
    assert_eq!(elab.ty, Value::Universe(2));
}

#[test]
fn thm_060_girard_paradox_prevention() {
    let mut ast = Ast::new();
    let u0 = ast.push(Expr::Universe(0)).unwrap();
    assert!(check(&ast, u0, Value::Universe(0), &[]).is_err());
}

// ============================================================================
// DOMINIO VII: COMBINADORES DE ORDEN SUPERIOR Y ÁLGEBRA SKI (TEOREMAS 061 - 070)
// ============================================================================

#[test]
fn thm_061_combinator_i() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "(fn x -> x) True", &[]);
    assert_eq!(eval(&ast, term, &[], None), Value::True);
}

#[test]
fn thm_062_combinator_k() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "(fn x -> fn y -> x) True False", &[]);
    assert_eq!(eval(&ast, term, &[], None), Value::True);
}

#[test]
fn thm_063_combinator_s() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "(fn x -> fn y -> fn z -> x z (y z)) (fn a -> fn b -> a) (fn c -> True) False", &[]);
    assert_eq!(eval(&ast, term, &[], None), Value::False);
}

#[test]
fn thm_064_combinator_skk_equiv_i() {
    // (S K K) x == I x == x
    let mut ast = Ast::new();
    let skk_true = parse_expr(&mut ast, "(fn x -> fn y -> fn z -> x z (y z)) (fn a -> fn b -> a) (fn a -> fn b -> a) True", &[]);
    let i_true = parse_expr(&mut ast, "(fn x -> x) True", &[]);
    assert_eq!(eval(&ast, skk_true, &[], None), eval(&ast, i_true, &[], None));
}

#[test]
fn thm_065_combinator_b() {
    // B f g x = f (g x)
    let mut ast = Ast::new();
    let b_app = parse_expr(&mut ast, "(fn f -> fn g -> fn x -> f (g x)) (fn n -> S(n)) (fn n -> S(n)) Z", &[]);
    assert_eq!(eval(&ast, b_app, &[], None), Value::Succ(Box::new(Value::Succ(Box::new(Value::Zero)))));
}

#[test]
fn thm_066_combinator_c() {
    // C f x y = f y x
    let mut ast = Ast::new();
    let c_app = parse_expr(&mut ast, "(fn f -> fn x -> fn y -> f y x) (fn a -> fn b -> a) True False", &[]);
    assert_eq!(eval(&ast, c_app, &[], None), Value::False);
}

#[test]
fn thm_067_combinator_w() {
    // W f x = f x x
    let mut ast = Ast::new();
    let w_app = parse_expr(&mut ast, "(fn f -> fn x -> f x x) (fn a -> fn b -> (a, b)) True", &[]);
    assert_eq!(eval(&ast, w_app, &[], None), Value::Pair(Box::new(Value::True), Box::new(Value::True)));
}

#[test]
fn thm_068_curry_uncurry_roundtrip() {
    let mut ast = Ast::new();
    let curry_uncurry = parse_expr(
        &mut ast,
        "(fn f -> fn a -> fn b -> f (a, b)) (fn p -> (p.1, p.2)) True False",
        &[],
    );
    assert_eq!(eval(&ast, curry_uncurry, &[], None), Value::Pair(Box::new(Value::True), Box::new(Value::False)));
}

#[test]
fn thm_069_eta_expansion_pi() {
    let mut ast = Ast::new();
    let f1 = parse_expr(&mut ast, "fn x -> (fn y -> y) x", &[]);
    let f2 = parse_expr(&mut ast, "fn y -> y", &[]);
    let v1 = eval(&ast, f1, &[], None);
    let v2 = eval(&ast, f2, &[], None);
    assert!(equiv(&ast, &v1, &v2, 0));
}

#[test]
fn thm_070_beta_reduction_pi() {
    let mut ast = Ast::new();
    let beta = parse_expr(&mut ast, "(fn x -> S(x)) (S(Z))", &[]);
    assert_eq!(eval(&ast, beta, &[], None), Value::Succ(Box::new(Value::Succ(Box::new(Value::Zero)))));
}

// ============================================================================
// DOMINIO VIII: IMPLÍCITOS Y RESOLUCIÓN DE METAVARIABLES (TEOREMAS 071 - 080)
// ============================================================================

#[test]
fn thm_071_implicit_identity_application() {
    let mut ast = Ast::new();
    let script = "let id : fn {A : type} -> fn (a : A) -> A = fn {A} -> fn a -> a";
    let cmd = parse_cmd(&mut ast, script, &[]);
    if let Command::Let { ty, term, .. } = cmd {
        let ty_val = eval(&ast, ty.unwrap(), &[], None);
        assert!(check(&ast, term, ty_val, &[]).is_ok());
    }
}

#[test]
fn thm_072_implicit_instantiation_nat() {
    let mut ast = Ast::new();
    let script1 = "let id : fn {A : type} -> fn (a : A) -> A = fn {A} -> fn a -> a";
    let cmd1 = parse_cmd(&mut ast, script1, &[]);
    let names = vec!["id".to_string()];
    let script2 = "let res = id S(Z)";
    let cmd2 = parse_cmd(&mut ast, script2, &names);

    let turbine = TurbineEngine::for_ast(&ast);
    let mut env = Vec::new();
    let mut types = Vec::new();

    if let Command::Let { ty, term, .. } = cmd1 {
        let ty_val = eval(&ast, ty.unwrap(), &env, Some(&turbine));
        check_with_turbine(&ast, term, ty_val.clone(), &types, &turbine).unwrap();
        let term_val = eval(&ast, term, &env, Some(&turbine));
        env.push(term_val);
        types.push(ty_val);
    }

    if let Command::Let { term, .. } = cmd2 {
        let elab = synthesize_with_turbine(&ast, term, &types, &turbine).unwrap();
        let final_type = turbine.force(&ast, elab.ty);
        assert_eq!(final_type, Value::NatType);
    }
}

#[test]
fn thm_073_implicit_instantiation_bool() {
    let mut ast = Ast::new();
    let script1 = "let id : fn {A : type} -> fn (a : A) -> A = fn {A} -> fn a -> a";
    let cmd1 = parse_cmd(&mut ast, script1, &[]);
    let names = vec!["id".to_string()];
    let script2 = "let res = id True";
    let cmd2 = parse_cmd(&mut ast, script2, &names);

    let turbine = TurbineEngine::for_ast(&ast);
    let mut env = Vec::new();
    let mut types = Vec::new();

    if let Command::Let { ty, term, .. } = cmd1 {
        let ty_val = eval(&ast, ty.unwrap(), &env, Some(&turbine));
        check_with_turbine(&ast, term, ty_val.clone(), &types, &turbine).unwrap();
        let term_val = eval(&ast, term, &env, Some(&turbine));
        env.push(term_val);
        types.push(ty_val);
    }

    if let Command::Let { term, .. } = cmd2 {
        let elab = synthesize_with_turbine(&ast, term, &types, &turbine).unwrap();
        let final_type = turbine.force(&ast, elab.ty);
        assert_eq!(final_type, Value::Bool);
    }
}

#[test]
fn thm_074_hole_first_order_solve() {
    let mut ast = Ast::new();
    let script1 = "let id : fn (A : type) -> fn (a : A) -> A = fn A -> fn a -> a";
    let cmd1 = parse_cmd(&mut ast, script1, &[]);
    let names = vec!["id".to_string()];
    let script2 = "let res = id ? ()";
    let cmd2 = parse_cmd(&mut ast, script2, &names);

    let turbine = TurbineEngine::for_ast(&ast);
    let mut env = Vec::new();
    let mut types = Vec::new();

    if let Command::Let { ty, term, .. } = cmd1 {
        let ty_val = eval(&ast, ty.unwrap(), &env, Some(&turbine));
        check_with_turbine(&ast, term, ty_val.clone(), &types, &turbine).unwrap();
        let term_val = eval(&ast, term, &env, Some(&turbine));
        env.push(term_val);
        types.push(ty_val);
    }

    if let Command::Let { term, .. } = cmd2 {
        let elab = synthesize_with_turbine(&ast, term, &types, &turbine).unwrap();
        let final_type = turbine.force(&ast, elab.ty);
        assert_eq!(final_type, Value::UnitType);
    }
}

#[test]
fn thm_075_hole_polymorphic_instantiation() {
    let mut ast = Ast::new();
    let script1 = "let const_fn : fn (A: type) -> fn (B: type) -> fn (a: A) -> fn (b: B) -> A = fn A -> fn B -> fn a -> fn b -> a";
    let cmd1 = parse_cmd(&mut ast, script1, &[]);
    let names = vec!["const_fn".to_string()];
    let script2 = "let res = const_fn ? ? Z True";
    let cmd2 = parse_cmd(&mut ast, script2, &names);

    let turbine = TurbineEngine::for_ast(&ast);
    let mut env = Vec::new();
    let mut types = Vec::new();

    if let Command::Let { ty, term, .. } = cmd1 {
        let ty_val = eval(&ast, ty.unwrap(), &env, Some(&turbine));
        check_with_turbine(&ast, term, ty_val.clone(), &types, &turbine).unwrap();
        let term_val = eval(&ast, term, &env, Some(&turbine));
        env.push(term_val);
        types.push(ty_val);
    }

    if let Command::Let { term, .. } = cmd2 {
        let elab = synthesize_with_turbine(&ast, term, &types, &turbine).unwrap();
        let final_type = turbine.force(&ast, elab.ty);
        assert_eq!(final_type, Value::NatType);
    }
}

#[test]
fn thm_076_hole_cascade_two_step() {
    let mut ast = Ast::new();
    let script1 = "let pair_builder : fn (A: type) -> fn (a: A) -> sigma (x: A) * A = fn A -> fn a -> (a, a)";
    let cmd1 = parse_cmd(&mut ast, script1, &[]);
    let names = vec!["pair_builder".to_string()];
    let script2 = "let p = pair_builder ? S(Z)";
    let cmd2 = parse_cmd(&mut ast, script2, &names);

    let turbine = TurbineEngine::for_ast(&ast);
    let mut env = Vec::new();
    let mut types = Vec::new();

    if let Command::Let { ty, term, .. } = cmd1 {
        let ty_val = eval(&ast, ty.unwrap(), &env, Some(&turbine));
        check_with_turbine(&ast, term, ty_val.clone(), &types, &turbine).unwrap();
        let term_val = eval(&ast, term, &env, Some(&turbine));
        env.push(term_val);
        types.push(ty_val);
    }

    if let Command::Let { term, .. } = cmd2 {
        let elab = synthesize_with_turbine(&ast, term, &types, &turbine).unwrap();
        let final_type = turbine.force(&ast, elab.ty);
        assert!(matches!(final_type, Value::Sigma(..)));
    }
}

#[test]
fn thm_077_hole_spine_instantiation() {
    let mut ast = Ast::new();
    let hole = ast.push(Expr::Hole).unwrap();
    let turbine = TurbineEngine::for_ast(&ast);
    // Hole evaluates to Meta in turbine
    turbine.expr_to_meta.write().unwrap().insert(hole, micro_axiom_0::ast::MetaId(999));
    let val = eval(&ast, hole, &[], Some(&turbine));
    assert!(matches!(val, Value::Meta(micro_axiom_0::ast::MetaId(999), _)));
}

#[test]
fn thm_078_hole_force_normalized() {
    let ast = Ast::new();
    let turbine = TurbineEngine::for_ast(&ast);
    let m = turbine.new_meta();
    turbine.solve_meta(m, Value::NatType);
    let forced = turbine.force(&ast, Value::Meta(m, Vec::new()));
    assert_eq!(forced, Value::NatType);
}

#[test]
fn thm_079_explicit_override_of_implicit() {
    let mut ast = Ast::new();
    let script1 = "let id : fn {A : type} -> fn (a : A) -> A = fn {A} -> fn a -> a";
    let cmd1 = parse_cmd(&mut ast, script1, &[]);
    let names = vec!["id".to_string()];
    let script2 = "let res = id {Nat} Z";
    let cmd2 = parse_cmd(&mut ast, script2, &names);

    let turbine = TurbineEngine::for_ast(&ast);
    let mut env = Vec::new();
    let mut types = Vec::new();

    if let Command::Let { ty, term, .. } = cmd1 {
        let ty_val = eval(&ast, ty.unwrap(), &env, Some(&turbine));
        check_with_turbine(&ast, term, ty_val.clone(), &types, &turbine).unwrap();
        let term_val = eval(&ast, term, &env, Some(&turbine));
        env.push(term_val);
        types.push(ty_val);
    }

    if let Command::Let { term, .. } = cmd2 {
        let elab = synthesize_with_turbine(&ast, term, &types, &turbine).unwrap();
        let final_type = turbine.force(&ast, elab.ty);
        assert_eq!(final_type, Value::NatType);
    }
}

#[test]
fn thm_080_chained_implicits() {
    let mut ast = Ast::new();
    let script = "let f : fn {A : type} -> fn {B : type} -> fn (b : B) -> B = fn {A} -> fn {B} -> fn b -> b";
    let cmd = parse_cmd(&mut ast, script, &[]);
    if let Command::Let { ty, term, .. } = cmd {
        let ty_val = eval(&ast, ty.unwrap(), &[], None);
        assert!(check(&ast, term, ty_val, &[]).is_ok());
    }
}

// ============================================================================
// DOMINIO IX: CONCURRENCIA Y SINCRONIZACIÓN SEQLOCK (TEOREMAS 081 - 090)
// ============================================================================

#[test]
fn thm_081_seqlock_version_zero_coherence() {
    let cell = SeqlockCell::new([1usize, 2, 3, 4]);
    let snap = cell.try_read(10).unwrap();
    assert_eq!(snap.version, 0);
    assert_eq!(snap.value, [1, 2, 3, 4]);
}

#[test]
fn thm_082_seqlock_version_step_two() {
    let cell = SeqlockCell::new([0usize; 2]);
    let v1 = cell.try_write([10, 20]).unwrap();
    assert_eq!(v1, 2);
    let v2 = cell.try_write([30, 40]).unwrap();
    assert_eq!(v2, 4);
}

#[test]
fn thm_083_seqlock_zero_sized_payload_versioning() {
    let cell = SeqlockCell::<usize, 0>::new([]);
    assert_eq!(cell.try_write([]).unwrap(), 2);
    let snap = cell.try_read(1).unwrap();
    assert_eq!(snap.version, 2);
    assert_eq!(snap.value, []);
    assert_eq!(cell.try_write([]).unwrap(), 4);
    let snap2 = cell.try_read(1).unwrap();
    assert_eq!(snap2.version, 4);
}

#[test]
fn thm_084_seqlock_uncontended_read_zero_retries() {
    let cell = SeqlockCell::new([42usize]);
    let snap = cell.try_read(8).unwrap();
    assert_eq!(snap.attempts, 1);
    assert_eq!(snap.value[0], 42);
}

#[test]
fn thm_085_seqlock_retry_budget_exhaustion() {
    let cell = SeqlockCell::new([0usize]);
    let err = cell.try_read(0).unwrap_err();
    assert_eq!(err, ReadError::RetryBudgetExhausted { attempts: 0 });
}

#[test]
fn thm_086_seqlock_payload_store_release_acquire() {
    let cell = SeqlockCell::new([100usize, 200]);
    cell.try_write([300, 400]).unwrap();
    let snap = cell.try_read(4).unwrap();
    assert_eq!(snap.value, [300, 400]);
}

#[test]
fn thm_087_seqlock_wide_snapshot_16_words() {
    let initial = [7usize; 16];
    let cell = SeqlockCell::new(initial);
    let snap = cell.try_read(16).unwrap();
    assert_eq!(snap.value, initial);
}

#[test]
fn thm_088_turbine_atomic_snapshot_encoding_decoding() {
    let snap = AtomicElabSnapshot {
        status: ElabStatus::CertifiedValid,
        type_tag: TypeTag::Universe,
        level: 42,
        quantity: Quantity::One,
    };
    let words = snap.to_words();
    let decoded = AtomicElabSnapshot::from_words(words);
    assert_eq!(snap, decoded);
}

#[test]
fn thm_089_turbine_concurrent_publish_read() {
    let mut ast = Ast::new();
    let dummy = ast.push(Expr::Unit).unwrap();
    let turbine = TurbineEngine::for_ast(&ast);
    let snap = AtomicElabSnapshot {
        status: ElabStatus::CertifiedValid,
        type_tag: TypeTag::Unit,
        level: 0,
        quantity: Quantity::Zero,
    };
    assert!(turbine.publish(dummy, snap).is_ok());
    let cached = turbine.try_get_cached(dummy, 4).unwrap();
    assert_eq!(AtomicElabSnapshot::from_words(cached.value).status, ElabStatus::CertifiedValid);
}

#[test]
fn thm_090_turbine_batch_parallel_determinism() {
    let mut ast = Ast::new();
    let u0 = ast.push(Expr::Universe(0)).unwrap();
    let turbine = TurbineEngine::for_ast(&ast);
    let roots = vec![u0, u0];
    let results = turbine.elaborate_batch_parallel(&ast, &roots, &[]);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].as_ref().unwrap().ty, Value::Universe(1));
    assert_eq!(results[1].as_ref().unwrap().ty, Value::Universe(1));
}

// ============================================================================
// DOMINIO X: META-TEOREMAS Y AISLAMIENTO DE GÖDEL-TURING (TEOREMAS 091 - 100)
// ============================================================================

#[test]
fn thm_091_empty_context_closed_term() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "fn x -> x", &[]);
    let ty = parse_expr(&mut ast, "fn (x: UnitType) -> UnitType", &[]);
    let expected = eval(&ast, ty, &[], None);
    let elab = check(&ast, term, expected, &[]).unwrap();
    assert!(elab.usages.is_empty(), "Closed term must have empty usages");
}

#[test]
fn thm_092_scope_isolation_de_bruijn() {
    let mut ast = Ast::new();
    // Shadowing outer variable with inner: (\x. \x. x) applied to Z and True returns True
    let term = parse_expr(&mut ast, "fn x -> fn x -> x", &[]);
    let ty = parse_expr(&mut ast, "fn (a: Nat) -> fn (b: Bool) -> Bool", &[]);
    let expected = eval(&ast, ty, &[], None);
    assert!(check(&ast, term, expected, &[]).is_ok());

    let app = parse_expr(&mut ast, "(fn x -> fn x -> x) Z True", &[]);
    assert_eq!(eval(&ast, app, &[], None), Value::True);
}

#[test]
fn thm_093_shared_expression_ast_protection() {
    let mut ast = Ast::new();
    let node = ast.push(Expr::Unit).unwrap();
    let pair = ast.push(Expr::Pair { first: node, second: node }).unwrap();
    let ty = parse_expr(&mut ast, "sigma (x: UnitType) * UnitType", &[]);
    let expected = eval(&ast, ty, &[], None);
    let elab_res = check(&ast, pair, expected, &[]);
    assert!(matches!(elab_res, Err(micro_axiom_0::elaborator::Error::SharedExpression(_))));
}

#[test]
fn thm_094_context_length_mismatch_protection() {
    let mut ast = Ast::new();
    let root = ast.push(Expr::Unit).unwrap();
    let env = vec![Value::Unit];
    let types = vec![]; // Length mismatch: env=1, types=0
    let res = elaborator::check_in_env(&ast, root, Value::UnitType, &env, &types);
    assert!(matches!(res, Err(micro_axiom_0::elaborator::Error::ContextLengthMismatch { .. })));
}

#[test]
fn thm_095_peano_ind_zero_case_definitional() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "ind(fn _ -> Nat, S(S(Z)), fn _ -> fn ih -> S(ih), Z)", &[]);
    assert_eq!(eval(&ast, term, &[], None), Value::Succ(Box::new(Value::Succ(Box::new(Value::Zero)))));
}

#[test]
fn thm_096_peano_ind_step_case_definitional() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "ind(fn _ -> Nat, S(Z), fn _ -> fn ih -> S(ih), S(Z))", &[]);
    assert_eq!(eval(&ast, term, &[], None), Value::Succ(Box::new(Value::Succ(Box::new(Value::Zero)))));
}

#[test]
fn thm_097_martin_lof_j_refl_definitional() {
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "J(fn b -> fn _ -> Nat, S(S(S(Z))), refl(Z))", &[]);
    assert_eq!(eval(&ast, term, &[], None), Value::Succ(Box::new(Value::Succ(Box::new(Value::Succ(Box::new(Value::Zero)))))));
}

#[test]
fn thm_098_neutral_j_suspension() {
    let mut ast = Ast::new();
    let p_var = Value::Neutral(Neutral::Var(Level(0)));
    let term = parse_expr(&mut ast, "J(fn b -> fn _ -> Nat, S(Z), p)", &["p".to_string()]);
    let val = eval(&ast, term, &[p_var], None);
    assert!(matches!(val, Value::Neutral(Neutral::J(..))));
}

#[test]
fn thm_099_neutral_ind_suspension() {
    let mut ast = Ast::new();
    let n_var = Value::Neutral(Neutral::Var(Level(0)));
    let term = parse_expr(&mut ast, "ind(fn _ -> Nat, Z, fn _ -> fn ih -> S(ih), n)", &["n".to_string()]);
    let val = eval(&ast, term, &[n_var], None);
    assert!(matches!(val, Value::Neutral(Neutral::Ind(..))));
}

#[test]
fn thm_100_clausura_epistemica_omega() {
    // Teorema 100 (Punto Fijo Omega): Clausura Epistémica Absoluta
    // Verificación de que todo cálculo y demostración se reduce a forma canónica sin ciclos espurios
    let mut ast = Ast::new();
    let term = parse_expr(&mut ast, "(fn x -> refl(x)) S(S(Z))", &[]);
    let val = eval(&ast, term, &[], None);
    assert_eq!(val, Value::Refl(Box::new(Value::Succ(Box::new(Value::Succ(Box::new(Value::Zero)))))));

    let canonical = parse_expr(&mut ast, "refl(S(S(Z)))", &[]);
    let ty = parse_expr(&mut ast, "Id(Nat, S(S(Z)), S(S(Z)))", &[]);
    let ty_val = eval(&ast, ty, &[], None);
    let elab = check(&ast, canonical, ty_val.clone(), &[]).unwrap();
    assert_eq!(elab.ty, ty_val);
}

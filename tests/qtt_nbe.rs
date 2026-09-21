use micro_axiom_0::ast::{Ast, Expr, Level, Quantity};
use micro_axiom_0::elaborator::{Error, check, synthesize};
use micro_axiom_0::eval::{Neutral, Value, equiv, eval};
use micro_axiom_0::parser::Parser;

#[test]
fn test_nbe_beta_reduction() {
    let mut ast = Ast::new();
    // Identity: fn x -> x
    let var0 = ast.push(Expr::Var(Level(0))).unwrap();
    let id_lam = ast
        .push(Expr::Lambda {
            quantity: Quantity::One,
            body: var0,
        })
        .unwrap();

    // Unit argument
    let unit_arg = ast.push(Expr::Unit).unwrap();

    // App: (fn x -> x) ()
    let app = ast
        .push(Expr::App {
            function: id_lam,
            argument: unit_arg,
        })
        .unwrap();

    let res = eval(&ast, app, &[], None);
    assert_eq!(res, Value::Unit);
}

#[test]
fn test_nbe_eta_equality() {
    let mut ast = Ast::new();
    // f is neutral variable at Level 0
    let f_val = Value::Neutral(Neutral::Var(Level(0)));

    // lam (\x. f x)
    let var0 = ast.push(Expr::Var(Level(0))).unwrap(); // f
    let var1 = ast.push(Expr::Var(Level(1))).unwrap(); // x
    let app = ast
        .push(Expr::App {
            function: var0,
            argument: var1,
        })
        .unwrap();
    let lam = ast
        .push(Expr::Lambda {
            quantity: Quantity::Omega,
            body: app,
        })
        .unwrap();

    let lam_val = eval(&ast, lam, std::slice::from_ref(&f_val), None);

    // By eta-equality: (\x. f x) == f
    assert!(equiv(&ast, &lam_val, &f_val, 1));
}

#[test]
fn test_universe_stratification() {
    let mut ast = Ast::new();
    let u0 = ast.push(Expr::Universe(0)).unwrap();
    let u1 = ast.push(Expr::Universe(1)).unwrap();

    // type 0 : type 1
    let elab0 = synthesize(&ast, u0, &[]).expect("synthesize type 0");
    assert_eq!(elab0.ty, Value::Universe(1));

    // type 1 : type 2
    let elab1 = synthesize(&ast, u1, &[]).expect("synthesize type 1");
    assert_eq!(elab1.ty, Value::Universe(2));
}

#[test]
fn test_girard_paradox_prevention() {
    let mut ast = Ast::new();
    let u0 = ast.push(Expr::Universe(0)).unwrap();

    // Checking type 0 against type 0 MUST fail (no type : type)
    let err = check(&ast, u0, Value::Universe(0), &[]);
    assert!(matches!(err, Err(Error::TypeMismatch { .. })));
}

#[test]
fn test_cumulative_subtyping() {
    let mut ast = Ast::new();
    let u0 = ast.push(Expr::Universe(0)).unwrap();

    // type 0 can be checked against type 1 (Type 0 <= Type 1)
    let elab = check(&ast, u0, Value::Universe(1), &[]);
    assert!(elab.is_ok());

    // type 0 can be checked against type 5
    let elab_deep = check(&ast, u0, Value::Universe(5), &[]);
    assert!(elab_deep.is_ok());

    // But type 2 cannot be checked against type 1
    let u2 = ast.push(Expr::Universe(2)).unwrap();
    let err = check(&ast, u2, Value::Universe(1), &[]);
    assert!(matches!(err, Err(Error::TypeMismatch { .. })));
}

#[test]
fn test_qtt_linear_variable_unused_fails() {
    let mut ast = Ast::new();
    // Type: fn :^1 (x: type 0) -> type 1
    let u0_dom = ast.push(Expr::Universe(0)).unwrap();
    let u1_cod = ast.push(Expr::Universe(1)).unwrap();
    let pi_ty = ast
        .push(Expr::Pi {
            quantity: Quantity::One,
            domain: u0_dom,
            codomain: u1_cod,
        })
        .unwrap();
    let expected = eval(&ast, pi_ty, &[], None);

    // Term: fn :^1 x -> type 0 (x is dropped / unused; type 0 has type type 1)
    let body = ast.push(Expr::Universe(0)).unwrap();
    let lam = ast
        .push(Expr::Lambda {
            quantity: Quantity::One,
            body,
        })
        .unwrap();

    let res = check(&ast, lam, expected, &[]);
    assert_eq!(
        res.unwrap_err(),
        Error::UsageMismatch {
            expr: lam,
            declared: Quantity::One,
            observed: Quantity::Zero,
        }
    );
}

#[test]
fn test_qtt_erased_variable_used_fails() {
    let mut ast = Ast::new();
    // Type: fn :^0 (x: type 0) -> type 0
    let u0_dom = ast.push(Expr::Universe(0)).unwrap();
    let u0_cod = ast.push(Expr::Universe(0)).unwrap();
    let pi_ty = ast
        .push(Expr::Pi {
            quantity: Quantity::Zero,
            domain: u0_dom,
            codomain: u0_cod,
        })
        .unwrap();
    let expected = eval(&ast, pi_ty, &[], None);

    // Term: fn :^0 x -> x (x is used, violating erased quantity 0; x has type type 0)
    let var0 = ast.push(Expr::Var(Level(0))).unwrap();
    let lam = ast
        .push(Expr::Lambda {
            quantity: Quantity::Zero,
            body: var0,
        })
        .unwrap();

    let res = check(&ast, lam, expected, &[]);
    assert_eq!(
        res.unwrap_err(),
        Error::UsageMismatch {
            expr: lam,
            declared: Quantity::Zero,
            observed: Quantity::One,
        }
    );
}

#[test]
fn test_qtt_valid_linear_and_erased() {
    let mut ast = Ast::new();
    // 1. Valid Linear Identity: fn :^1 (x: type 0) -> type 0 = fn :^1 x -> x
    let u0_dom = ast.push(Expr::Universe(0)).unwrap();
    let u0_cod = ast.push(Expr::Universe(0)).unwrap();
    let pi_linear = ast
        .push(Expr::Pi {
            quantity: Quantity::One,
            domain: u0_dom,
            codomain: u0_cod,
        })
        .unwrap();
    let exp_linear = eval(&ast, pi_linear, &[], None);

    let var0 = ast.push(Expr::Var(Level(0))).unwrap();
    let lam_linear = ast
        .push(Expr::Lambda {
            quantity: Quantity::One,
            body: var0,
        })
        .unwrap();
    let elab_linear = check(&ast, lam_linear, exp_linear, &[]);
    assert!(elab_linear.is_ok());

    // 2. Valid Erased Const: fn :^0 (x: type 0) -> type 1 = fn :^0 x -> type 0
    let u0_dom2 = ast.push(Expr::Universe(0)).unwrap();
    let u1_cod2 = ast.push(Expr::Universe(1)).unwrap();
    let pi_erased = ast
        .push(Expr::Pi {
            quantity: Quantity::Zero,
            domain: u0_dom2,
            codomain: u1_cod2,
        })
        .unwrap();
    let exp_erased = eval(&ast, pi_erased, &[], None);

    let u0_body = ast.push(Expr::Universe(0)).unwrap();
    let lam_erased = ast
        .push(Expr::Lambda {
            quantity: Quantity::Zero,
            body: u0_body,
        })
        .unwrap();
    let elab_erased = check(&ast, lam_erased, exp_erased, &[]);
    assert!(elab_erased.is_ok());
}

#[test]
fn test_parser_and_repl_commands() {
    use micro_axiom_0::lexer::Lexer;
    let mut ast = Ast::new();
    let src = "let id : fn :^1 (x: type 0) -> type 0 = fn :^1 x -> x;";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize_all().expect("tokenize");
    let mut parser = Parser::new(&tokens, &mut ast, &[]);
    let cmd = parser.parse_command().expect("parse let command");
    match cmd {
        micro_axiom_0::ast::Command::Let { name, ty, term } => {
            assert_eq!(name, "id");
            assert!(ty.is_some());
            let ty_expr = ty.unwrap();
            let elab_ty = synthesize(&ast, ty_expr, &[]).expect("synthesize type");
            assert_eq!(elab_ty.ty, Value::Universe(1));

            let exp_val = eval(&ast, ty_expr, &[], None);
            let elab_term = check(&ast, term, exp_val, &[]).expect("check term");
            assert!(elab_term.usages.is_empty());
        }
        _ => panic!("Expected Command::Let"),
    }
}

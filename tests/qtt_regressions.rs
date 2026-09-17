use micro_axiom_0::ast::{Ast, Expr, ExprId, Level, Quantity};
use micro_axiom_0::elaborator::{Error, check, synthesize};
use micro_axiom_0::eval::{Value, eval};

fn var(ast: &mut Ast, level: usize) -> ExprId {
    ast.push(Expr::Var(Level(level))).unwrap()
}

fn lambda(ast: &mut Ast, quantity: Quantity, body: ExprId) -> ExprId {
    ast.push(Expr::Lambda { quantity, body }).unwrap()
}

fn app(ast: &mut Ast, function: ExprId, argument: ExprId) -> ExprId {
    ast.push(Expr::App { function, argument }).unwrap()
}

fn pi(ast: &mut Ast, quantity: Quantity, domain: ExprId, codomain: ExprId) -> ExprId {
    ast.push(Expr::Pi {
        quantity,
        domain,
        codomain,
    })
    .unwrap()
}

fn unit_function(ast: &mut Ast, quantity: Quantity) -> ExprId {
    let domain = ast.push(Expr::UnitType).unwrap();
    let codomain = ast.push(Expr::UnitType).unwrap();
    pi(ast, quantity, domain, codomain)
}

fn closure_consumer(ast: &mut Ast, quantity: Quantity) -> Value {
    let domain = unit_function(ast, Quantity::Zero);
    let codomain = ast.push(Expr::UnitType).unwrap();
    let function = pi(ast, quantity, domain, codomain);
    eval(ast, function, &[])
}

fn two_closure_consumer(ast: &mut Ast, parameter_quantity: Quantity) -> Value {
    let first = unit_function(ast, parameter_quantity);
    let second = unit_function(ast, parameter_quantity);
    let result = ast.push(Expr::UnitType).unwrap();
    let tail = pi(ast, Quantity::One, second, result);
    let function = pi(ast, Quantity::One, first, tail);
    eval(ast, function, &[])
}

#[test]
fn application_scales_a_closures_free_capture_by_the_parameter_quantity() {
    for quantity in [Quantity::Zero, Quantity::One, Quantity::Omega] {
        let mut ast = Ast::new();
        let consumer_type = closure_consumer(&mut ast, quantity);
        // f (fn :^0 ignored -> captured), in the context f, captured.
        let function = var(&mut ast, 0);
        let capture = var(&mut ast, 1);
        let argument = lambda(&mut ast, Quantity::Zero, capture);
        let term = app(&mut ast, function, argument);

        let result = synthesize(&ast, term, &[consumer_type, Value::UnitType]).unwrap();

        assert_eq!(result.ty, Value::UnitType);
        assert_eq!(result.usages, vec![Quantity::One, quantity], "{quantity}");
    }
}

#[test]
fn unrestricted_argument_cannot_capture_a_linear_outer_binder() {
    let mut ast = Ast::new();
    let consumer_type = closure_consumer(&mut ast, Quantity::Omega);
    let expected = unit_function(&mut ast, Quantity::One);
    let expected = eval(&ast, expected, &[]);
    let function = var(&mut ast, 0);
    let capture = var(&mut ast, 1);
    let argument = lambda(&mut ast, Quantity::Zero, capture);
    let body = app(&mut ast, function, argument);
    let term = lambda(&mut ast, Quantity::One, body);

    assert_eq!(
        check(&ast, term, expected, &[consumer_type]).unwrap_err(),
        Error::UsageMismatch {
            expr: term,
            declared: Quantity::One,
            observed: Quantity::Omega,
        }
    );
}

#[test]
fn erased_argument_can_capture_an_erased_outer_binder() {
    let mut ast = Ast::new();
    let consumer_type = closure_consumer(&mut ast, Quantity::Zero);
    let expected = unit_function(&mut ast, Quantity::Zero);
    let expected = eval(&ast, expected, &[]);
    let function = var(&mut ast, 0);
    let capture = var(&mut ast, 1);
    let argument = lambda(&mut ast, Quantity::Zero, capture);
    let body = app(&mut ast, function, argument);
    let term = lambda(&mut ast, Quantity::Zero, body);

    let result = check(&ast, term, expected, &[consumer_type]).unwrap();

    assert_eq!(result.usages, vec![Quantity::One]);
}

#[test]
fn erased_argument_does_not_consume_a_linear_outer_binder() {
    let mut ast = Ast::new();
    let consumer_type = closure_consumer(&mut ast, Quantity::Zero);
    let expected = unit_function(&mut ast, Quantity::One);
    let expected = eval(&ast, expected, &[]);
    let function = var(&mut ast, 0);
    let capture = var(&mut ast, 1);
    let argument = lambda(&mut ast, Quantity::Zero, capture);
    let body = app(&mut ast, function, argument);
    let term = lambda(&mut ast, Quantity::One, body);

    assert_eq!(
        check(&ast, term, expected, &[consumer_type]).unwrap_err(),
        Error::UsageMismatch {
            expr: term,
            declared: Quantity::One,
            observed: Quantity::Zero,
        }
    );
}

#[test]
fn duplicate_capture_in_sibling_arguments_rejects_a_linear_outer_binder() {
    let mut ast = Ast::new();
    let consumer_type = two_closure_consumer(&mut ast, Quantity::Zero);
    let expected = unit_function(&mut ast, Quantity::One);
    let expected = eval(&ast, expected, &[]);
    let function = var(&mut ast, 0);
    let first_capture = var(&mut ast, 1);
    let first = lambda(&mut ast, Quantity::Zero, first_capture);
    let second_capture = var(&mut ast, 1);
    let second = lambda(&mut ast, Quantity::Zero, second_capture);
    let partial = app(&mut ast, function, first);
    let body = app(&mut ast, partial, second);
    let term = lambda(&mut ast, Quantity::One, body);

    assert_eq!(
        check(&ast, term, expected, &[consumer_type]).unwrap_err(),
        Error::UsageMismatch {
            expr: term,
            declared: Quantity::One,
            observed: Quantity::Omega,
        }
    );
}

#[test]
fn sibling_lambdas_keep_distinct_captures_at_their_original_levels() {
    let mut ast = Ast::new();
    let consumer_type = two_closure_consumer(&mut ast, Quantity::Zero);
    let function = var(&mut ast, 0);
    let first_capture = var(&mut ast, 1);
    let first = lambda(&mut ast, Quantity::Zero, first_capture);
    let second_capture = var(&mut ast, 2);
    let second = lambda(&mut ast, Quantity::Zero, second_capture);
    let partial = app(&mut ast, function, first);
    let term = app(&mut ast, partial, second);

    let result = synthesize(
        &ast,
        term,
        &[consumer_type, Value::UnitType, Value::UnitType],
    )
    .unwrap();

    assert_eq!(result.usages, vec![Quantity::One; 3]);
}

#[test]
fn sibling_linear_binders_do_not_leak_into_the_enclosing_usage_vector() {
    let mut ast = Ast::new();
    let consumer_type = two_closure_consumer(&mut ast, Quantity::One);
    let function = var(&mut ast, 0);
    let first_local = var(&mut ast, 1);
    let first = lambda(&mut ast, Quantity::One, first_local);
    let second_local = var(&mut ast, 1);
    let second = lambda(&mut ast, Quantity::One, second_local);
    let partial = app(&mut ast, function, first);
    let term = app(&mut ast, partial, second);

    let result = synthesize(&ast, term, &[consumer_type]).unwrap();

    assert_eq!(result.usages, vec![Quantity::One]);
}

#[test]
fn erasing_an_argument_does_not_hide_a_discarded_linear_binder_inside_it() {
    let mut ast = Ast::new();
    let domain = unit_function(&mut ast, Quantity::One);
    let codomain = ast.push(Expr::UnitType).unwrap();
    let consumer = pi(&mut ast, Quantity::Zero, domain, codomain);
    let consumer_type = eval(&ast, consumer, &[]);
    let function = var(&mut ast, 0);
    let ignored_body = ast.push(Expr::Unit).unwrap();
    let argument = lambda(&mut ast, Quantity::One, ignored_body);
    let term = app(&mut ast, function, argument);

    assert_eq!(
        synthesize(&ast, term, &[consumer_type]).unwrap_err(),
        Error::UsageMismatch {
            expr: argument,
            declared: Quantity::One,
            observed: Quantity::Zero,
        }
    );
}

#[test]
fn dependent_annotation_does_not_turn_an_erased_type_binder_into_a_capture() {
    let mut ast = Ast::new();
    // fn :^0 (A : type 0) -> fn :^1 (x : A) -> A
    let universe = ast.push(Expr::Universe(0)).unwrap();
    let domain = var(&mut ast, 0);
    let codomain = var(&mut ast, 0);
    let inner_type = pi(&mut ast, Quantity::One, domain, codomain);
    let outer_type = pi(&mut ast, Quantity::Zero, universe, inner_type);
    synthesize(&ast, outer_type, &[]).unwrap();
    let expected = eval(&ast, outer_type, &[]);

    // fn :^0 A -> fn :^1 x -> (x : A)
    let value = var(&mut ast, 1);
    let annotation = var(&mut ast, 0);
    let body = ast
        .push(Expr::Ann {
            term: value,
            ty: annotation,
        })
        .unwrap();
    let inner = lambda(&mut ast, Quantity::One, body);
    let term = lambda(&mut ast, Quantity::Zero, inner);

    let result = check(&ast, term, expected, &[]).unwrap();

    assert!(result.usages.is_empty());
}

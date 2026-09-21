use micro_axiom_0::ast::{Ast, Expr, ExprId, Level, Quantity};
use micro_axiom_0::elaborator::{Error, check, check_in_env, synthesize, synthesize_in_env};
use micro_axiom_0::eval::{Neutral, Value, equiv, eval};

fn variable(ast: &mut Ast, level: usize) -> ExprId {
    ast.push(Expr::Var(Level(level))).unwrap()
}

fn alias_identity(ast: &mut Ast) -> (ExprId, ExprId, ExprId) {
    // With global A at level 0: (fn x -> (x : A)) : (x : A) -> A.
    let domain = variable(ast, 0);
    let codomain = variable(ast, 0);
    let ty = ast
        .push(Expr::Pi {
            quantity: Quantity::One,
            domain,
            codomain,
        })
        .unwrap();
    let term = variable(ast, 1);
    let body_ty = variable(ast, 0);
    let body = ast.push(Expr::Ann { term, ty: body_ty }).unwrap();
    let lambda = ast
        .push(Expr::Lambda {
            quantity: Quantity::One,
            body,
        })
        .unwrap();
    let annotated = ast.push(Expr::Ann { term: lambda, ty }).unwrap();
    (annotated, lambda, ty)
}

#[test]
fn global_unit_alias_supports_an_annotated_identity_and_application() {
    let mut ast = Ast::new();
    let (identity, lambda, identity_ty) = alias_identity(&mut ast);
    let argument = ast.push(Expr::Unit).unwrap();
    let application = ast
        .push(Expr::App {
            function: identity,
            argument,
        })
        .unwrap();
    let env = [Value::UnitType];
    let types = [Value::Universe(0)];

    let checked = check_in_env(&ast, lambda, eval(&ast, identity_ty, &env, None), &env, &types)
        .expect("the identity body resolves its global type alias");
    assert_eq!(checked.usages, vec![Quantity::Zero]);

    let result = synthesize_in_env(&ast, application, &env, &types)
        .expect("the identity accepts a value of the aliased type");
    assert_eq!(result.ty, Value::UnitType);
    assert_eq!(result.usages, vec![Quantity::Zero]);
    assert_eq!(eval(&ast, application, &env, None), Value::Unit);
}

#[test]
fn a_pi_domain_can_resolve_a_global_alias_for_a_universe() {
    let mut ast = Ast::new();
    // With U = type 0: (T : U) -> T is itself a type.
    let domain = variable(&mut ast, 0);
    let codomain = variable(&mut ast, 1);
    let pi = ast
        .push(Expr::Pi {
            quantity: Quantity::Zero,
            domain,
            codomain,
        })
        .unwrap();
    let result = synthesize_in_env(&ast, pi, &[Value::Universe(0)], &[Value::Universe(1)])
        .expect("the bound T receives the evaluated domain type 0");
    assert_eq!(result.ty, Value::Universe(1));
    assert!(result.usages.is_empty());

    assert!(matches!(
        synthesize(&ast, pi, &[Value::Universe(1)]),
        Err(Error::TypeMismatch { expr, .. }) if expr == codomain
    ));
}

#[test]
fn neutral_contexts_keep_global_slots_before_dependent_lambda_binders() {
    let mut ast = Ast::new();
    // Two existing globals precede T at level 2 and x at level 3.
    // The checked term is fn T -> fn x -> (x : T).
    let outer_domain = ast.push(Expr::Universe(0)).unwrap();
    let inner_domain = variable(&mut ast, 2);
    let inner_codomain = variable(&mut ast, 2);
    let inner_pi = ast
        .push(Expr::Pi {
            quantity: Quantity::One,
            domain: inner_domain,
            codomain: inner_codomain,
        })
        .unwrap();
    let outer_pi = ast
        .push(Expr::Pi {
            quantity: Quantity::Zero,
            domain: outer_domain,
            codomain: inner_pi,
        })
        .unwrap();
    let term = variable(&mut ast, 3);
    let ty = variable(&mut ast, 2);
    let body = ast.push(Expr::Ann { term, ty }).unwrap();
    let inner_lambda = ast
        .push(Expr::Lambda {
            quantity: Quantity::One,
            body,
        })
        .unwrap();
    let outer_lambda = ast
        .push(Expr::Lambda {
            quantity: Quantity::Zero,
            body: inner_lambda,
        })
        .unwrap();
    let types = [Value::Universe(0), Value::UnitType];
    let neutral_env = [
        Value::Neutral(Neutral::Var(Level(0))),
        Value::Neutral(Neutral::Var(Level(1))),
    ];
    let expected = eval(&ast, outer_pi, &neutral_env, None);

    let checked = check(&ast, outer_lambda, expected.clone(), &types)
        .expect("nested binders must retain their original De Bruijn levels");
    assert_eq!(checked.usages, vec![Quantity::Zero, Quantity::Zero]);
    assert!(equiv(&ast, &checked.ty, &expected, types.len()));

    let annotated = ast
        .push(Expr::Ann {
            term: outer_lambda,
            ty: outer_pi,
        })
        .unwrap();
    let synthesized = synthesize(&ast, annotated, &types)
        .expect("synthesis also retains global slots through nested binders");
    assert_eq!(synthesized.usages, vec![Quantity::Zero, Quantity::Zero]);
    assert!(equiv(&ast, &synthesized.ty, &expected, types.len()));
}

#[test]
fn neutral_wrappers_preserve_an_abstract_alias_instead_of_unfolding_it() {
    let mut ast = Ast::new();
    let (identity, lambda, ty) = alias_identity(&mut ast);
    let types = [Value::Universe(0)];
    let neutral_alias = Value::Neutral(Neutral::Var(Level(0)));
    let neutral_env = [neutral_alias.clone()];
    let abstract_ty = eval(&ast, ty, &neutral_env, None);

    let synthesized = synthesize(&ast, identity, &types).expect("identity at abstract A");
    assert!(equiv(&ast, &synthesized.ty, &abstract_ty, 1));
    match &synthesized.ty {
        Value::Pi(_, domain, codomain) => {
            assert_eq!(domain.as_ref(), &neutral_alias);
            assert_eq!(
                codomain.clone().instantiate(&ast, Value::Unit, None),
                neutral_alias
            );
        }
        other => panic!("expected an abstract function type, got {other:?}"),
    }
    check(&ast, lambda, abstract_ty, &types).expect("checking uses the same neutral context");

    let concrete = synthesize_in_env(&ast, identity, &[Value::UnitType], &types)
        .expect("the explicit environment unfolds A to UnitType");
    assert!(!equiv(&ast, &synthesized.ty, &concrete.ty, 1));
}

#[test]
fn a_concrete_alias_accepts_unit_but_an_abstract_alias_does_not() {
    let mut ast = Ast::new();
    let term = ast.push(Expr::Unit).unwrap();
    let ty = variable(&mut ast, 0);
    let annotated = ast.push(Expr::Ann { term, ty }).unwrap();
    let types = [Value::Universe(0)];

    assert_eq!(
        synthesize_in_env(&ast, annotated, &[Value::UnitType], &types)
            .expect("Unit inhabits the concrete alias")
            .ty,
        Value::UnitType
    );
    check_in_env(&ast, annotated, Value::UnitType, &[Value::UnitType], &types)
        .expect("checking also unfolds the alias");
    assert!(matches!(
        synthesize(&ast, annotated, &types),
        Err(Error::TypeMismatch { expr, .. }) if expr == term
    ));
    assert!(matches!(
        check(&ast, annotated, Value::UnitType, &types),
        Err(Error::TypeMismatch { expr, .. }) if expr == term
    ));
}

#[test]
fn explicit_context_apis_reject_different_numbers_of_values_and_types() {
    let mut ast = Ast::new();
    let unit = ast.push(Expr::Unit).unwrap();

    for (env, types) in [
        (vec![], vec![Value::UnitType]),
        (vec![Value::Unit], vec![]),
        (vec![Value::Unit, Value::Unit], vec![Value::UnitType]),
    ] {
        let expected = Error::ContextLengthMismatch {
            values: env.len(),
            types: types.len(),
        };
        assert_eq!(
            synthesize_in_env(&ast, unit, &env, &types).unwrap_err(),
            expected
        );
        assert_eq!(
            check_in_env(&ast, unit, Value::UnitType, &env, &types).unwrap_err(),
            expected
        );
    }
}

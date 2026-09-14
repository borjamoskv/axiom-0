use micro_axiom_0::{
    ast::{Ast, Expr, ExprId, Level, Quantity, Type, TypeId},
    elaborator::{Error, check, synthesize},
};

const QUANTITIES: [Quantity; 3] = [Quantity::Zero, Quantity::One, Quantity::Omega];

fn node(ast: &mut Ast, expr: Expr) -> ExprId {
    ast.push(expr).unwrap()
}

fn unit(ast: &mut Ast) -> ExprId {
    node(ast, Expr::Unit)
}

fn var(ast: &mut Ast, level: usize) -> ExprId {
    node(ast, Expr::Var(Level(level)))
}

fn lambda(ast: &mut Ast, quantity: Quantity, body: ExprId) -> ExprId {
    node(ast, Expr::Lambda { quantity, body })
}

fn app(ast: &mut Ast, function: ExprId, argument: ExprId) -> ExprId {
    node(ast, Expr::App { function, argument })
}

fn ann(ast: &mut Ast, term: ExprId, ty: TypeId) -> ExprId {
    node(ast, Expr::Ann { term, ty })
}

fn arrow(ast: &mut Ast, quantity: Quantity, domain: TypeId, codomain: TypeId) -> TypeId {
    ast.function_type(quantity, domain, codomain).unwrap()
}

fn consumer(ast: &mut Ast, quantity: Quantity, domain: TypeId) -> ExprId {
    let ty = arrow(ast, quantity, domain, ast.unit_type());
    let body = unit(ast);
    let term = lambda(ast, quantity, body);
    ann(ast, term, ty)
}

#[test]
fn quantities_obey_all_semiring_laws() {
    use Quantity::{One, Zero};
    for a in QUANTITIES {
        assert_eq!(a.plus(Zero), a);
        assert_eq!(a.times(One), a);
        assert_eq!(a.times(Zero), Zero);
        for b in QUANTITIES {
            assert_eq!(a.plus(b), b.plus(a));
            assert_eq!(a.times(b), b.times(a));
            for c in QUANTITIES {
                assert_eq!(a.plus(b).plus(c), a.plus(b.plus(c)));
                assert_eq!(a.times(b).times(c), a.times(b.times(c)));
                assert_eq!(a.times(b.plus(c)), a.times(b).plus(a.times(c)));
                assert_eq!(a.plus(b).times(c), a.times(c).plus(b.times(c)));
            }
        }
    }
    assert_eq!(One.plus(One), Quantity::Omega);
}

#[test]
fn erased_and_linear_permissions_are_exact() {
    for declared in QUANTITIES {
        for observed in QUANTITIES {
            assert_eq!(
                declared.permits(observed),
                declared == Quantity::Omega || declared == observed
            );
        }
    }
}

#[test]
fn function_types_are_canonical_and_preserve_quantity() {
    let mut ast = Ast::new();
    let u = ast.unit_type();
    let linear = arrow(&mut ast, Quantity::One, u, u);
    assert_eq!(linear, arrow(&mut ast, Quantity::One, u, u));
    assert_ne!(linear, arrow(&mut ast, Quantity::Zero, u, u));
    assert_ne!(linear, arrow(&mut ast, Quantity::Omega, u, u));
    let higher = arrow(&mut ast, Quantity::One, linear, u);
    assert_eq!(higher, arrow(&mut ast, Quantity::One, linear, u));
    assert_eq!(
        ast.ty(higher).unwrap(),
        Type::Function {
            quantity: Quantity::One,
            domain: linear,
            codomain: u
        }
    );
}

#[test]
fn bare_lambdas_check_and_annotated_lambdas_synthesize() {
    let mut ast = Ast::new();
    let u = ast.unit_type();
    let ty = arrow(&mut ast, Quantity::One, u, u);
    let body = var(&mut ast, 0);
    let identity = lambda(&mut ast, Quantity::One, body);
    assert_eq!(
        synthesize(&ast, identity),
        Err(Error::CannotInferLambda(identity))
    );
    assert_eq!(check(&ast, identity, ty).unwrap().ty, ty);
    let annotated = ann(&mut ast, identity, ty);
    let result = synthesize(&ast, annotated).unwrap();
    assert_eq!(result.ty, ty);
    assert_eq!(result.visited_nodes, 3);
    let checked = check(&ast, annotated, ty).unwrap();
    assert_eq!(checked.ty, result.ty);
    assert_eq!(checked.visited_nodes, result.visited_nodes);
    for expr in [body, identity, annotated] {
        assert_eq!(checked.type_of(expr), result.type_of(expr));
    }
}

#[test]
fn unused_parameters_require_zero_or_omega() {
    for quantity in QUANTITIES {
        let mut ast = Ast::new();
        let u = ast.unit_type();
        let ty = arrow(&mut ast, quantity, u, u);
        let body = unit(&mut ast);
        let root = lambda(&mut ast, quantity, body);
        if quantity == Quantity::One {
            assert_eq!(
                check(&ast, root, ty),
                Err(Error::UsageMismatch {
                    expr: root,
                    declared: Quantity::One,
                    observed: Quantity::Zero,
                })
            );
        } else {
            assert_eq!(check(&ast, root, ty).unwrap().ty, ty);
        }
    }
}

#[test]
fn runtime_occurrences_cannot_use_erased_parameters() {
    let mut ast = Ast::new();
    let u = ast.unit_type();
    let ty = arrow(&mut ast, Quantity::Zero, u, u);
    let body = var(&mut ast, 0);
    let root = lambda(&mut ast, Quantity::Zero, body);
    assert_eq!(
        check(&ast, root, ty),
        Err(Error::UsageMismatch {
            expr: root,
            declared: Quantity::Zero,
            observed: Quantity::One,
        })
    );
}

#[test]
fn contraction_requires_omega() {
    for quantity in [Quantity::One, Quantity::Omega] {
        let mut ast = Ast::new();
        let u = ast.unit_type();
        let linear = arrow(&mut ast, Quantity::One, u, u);
        let ty = arrow(&mut ast, quantity, linear, u);
        let first_f = var(&mut ast, 0);
        let second_f = var(&mut ast, 0);
        let value = unit(&mut ast);
        let inner = app(&mut ast, second_f, value);
        let body = app(&mut ast, first_f, inner);
        let root = lambda(&mut ast, quantity, body);
        if quantity == Quantity::One {
            assert_eq!(
                check(&ast, root, ty),
                Err(Error::UsageMismatch {
                    expr: root,
                    declared: quantity,
                    observed: Quantity::Omega,
                })
            );
        } else {
            assert_eq!(check(&ast, root, ty).unwrap().ty, ty);
        }
    }
}

#[test]
fn intrinsic_linearity_survives_zero_and_omega_surroundings() {
    for demand in [Quantity::Zero, Quantity::Omega] {
        let mut ast = Ast::new();
        let u = ast.unit_type();
        let linear = arrow(&mut ast, Quantity::One, u, u);
        let domain = arrow(&mut ast, Quantity::One, linear, u);
        let discard = consumer(&mut ast, demand, domain);
        let first_f = var(&mut ast, 0);
        let second_f = var(&mut ast, 0);
        let value = unit(&mut ast);
        let inner = app(&mut ast, second_f, value);
        let body = app(&mut ast, first_f, inner);
        let invalid = lambda(&mut ast, Quantity::One, body);
        let root = app(&mut ast, discard, invalid);
        assert_eq!(
            synthesize(&ast, root),
            Err(Error::UsageMismatch {
                expr: invalid,
                declared: Quantity::One,
                observed: Quantity::Omega,
            })
        );
    }
}

#[test]
fn returning_a_closure_consumes_its_linear_capture_once() {
    let mut ast = Ast::new();
    let u = ast.unit_type();
    let closure_ty = arrow(&mut ast, Quantity::Zero, u, u);
    let ty = arrow(&mut ast, Quantity::One, u, closure_ty);
    let captured = var(&mut ast, 0);
    let closure = lambda(&mut ast, Quantity::Zero, captured);
    let root = lambda(&mut ast, Quantity::One, closure);
    assert_eq!(check(&ast, root, ty).unwrap().ty, ty);
}

#[test]
fn duplicating_a_closure_cannot_duplicate_a_linear_capture() {
    let mut ast = Ast::new();
    let u = ast.unit_type();
    let closure_ty = arrow(&mut ast, Quantity::Zero, u, u);
    let ty = arrow(&mut ast, Quantity::One, u, u);
    let discard = consumer(&mut ast, Quantity::Omega, closure_ty);
    let captured = var(&mut ast, 0);
    let closure = lambda(&mut ast, Quantity::Zero, captured);
    let body = app(&mut ast, discard, closure);
    let root = lambda(&mut ast, Quantity::One, body);
    assert_eq!(
        check(&ast, root, ty),
        Err(Error::UsageMismatch {
            expr: root,
            declared: Quantity::One,
            observed: Quantity::Omega,
        })
    );
}

#[test]
fn zero_absorbs_omega_and_erased_uses_do_not_satisfy_linearity() {
    for quantity in [Quantity::Zero, Quantity::One] {
        let mut ast = Ast::new();
        let u = ast.unit_type();
        let ty = arrow(&mut ast, quantity, u, u);
        let erased = consumer(&mut ast, Quantity::Zero, u);
        let unrestricted = consumer(&mut ast, Quantity::Omega, u);
        let x = var(&mut ast, 0);
        let inner = app(&mut ast, unrestricted, x);
        let body = app(&mut ast, erased, inner);
        let root = lambda(&mut ast, quantity, body);
        if quantity == Quantity::One {
            assert_eq!(
                check(&ast, root, ty),
                Err(Error::UsageMismatch {
                    expr: root,
                    declared: quantity,
                    observed: Quantity::Zero,
                })
            );
        } else {
            assert_eq!(check(&ast, root, ty).unwrap().ty, ty);
        }
    }
}

#[test]
fn erased_argument_demand_does_not_leak_to_its_sibling() {
    let mut ast = Ast::new();
    let u = ast.unit_type();
    let linear = arrow(&mut ast, Quantity::One, u, u);
    let f_ty = arrow(&mut ast, Quantity::Zero, u, linear);
    // Under x(level 0), F = λ⁰e. λ¹y.y has y at level 2.
    let y = var(&mut ast, 2);
    let identity = lambda(&mut ast, Quantity::One, y);
    let f = lambda(&mut ast, Quantity::Zero, identity);
    let f = ann(&mut ast, f, f_ty);
    let erased_x = var(&mut ast, 0);
    let partial = app(&mut ast, f, erased_x);
    let runtime_x = var(&mut ast, 0);
    let body = app(&mut ast, partial, runtime_x);
    let root = lambda(&mut ast, Quantity::One, body);
    let result = check(&ast, root, linear).unwrap();
    assert_eq!(result.ty, linear);
    assert_eq!(result.visited_nodes, ast.expression_count());
}

#[test]
fn omega_argument_demand_does_not_leak_to_its_sibling() {
    let mut ast = Ast::new();
    let u = ast.unit_type();
    let linear = arrow(&mut ast, Quantity::One, u, u);
    let f_ty = arrow(&mut ast, Quantity::Omega, u, linear);
    let ty = arrow(&mut ast, Quantity::Omega, u, linear);
    // Under x(level 0), y(level 1), F = λωe. λ¹z.z has z at level 3.
    let z = var(&mut ast, 3);
    let identity = lambda(&mut ast, Quantity::One, z);
    let f = lambda(&mut ast, Quantity::Omega, identity);
    let f = ann(&mut ast, f, f_ty);
    let x = var(&mut ast, 0);
    let partial = app(&mut ast, f, x);
    let y = var(&mut ast, 1);
    let body = app(&mut ast, partial, y);
    let inner = lambda(&mut ast, Quantity::One, body);
    let root = lambda(&mut ast, Quantity::Omega, inner);
    assert_eq!(check(&ast, root, ty).unwrap().ty, ty);
}

#[test]
fn quantity_and_type_mismatches_are_reported() {
    let mut ast = Ast::new();
    let u = ast.unit_type();
    let linear = arrow(&mut ast, Quantity::One, u, u);
    let value = unit(&mut ast);
    let wrong_quantity = lambda(&mut ast, Quantity::Omega, value);
    assert_eq!(
        check(&ast, wrong_quantity, linear),
        Err(Error::QuantityMismatch {
            expr: wrong_quantity,
            expected: Quantity::One,
            found: Quantity::Omega,
        })
    );
    assert_eq!(
        check(&ast, value, linear),
        Err(Error::TypeMismatch {
            expr: value,
            expected: linear,
            found: u,
        })
    );
    assert_eq!(
        check(&ast, wrong_quantity, u),
        Err(Error::ExpectedFunction {
            expr: wrong_quantity,
            found: u,
        })
    );
    let argument = unit(&mut ast);
    let invalid_app = app(&mut ast, value, argument);
    assert_eq!(
        synthesize(&ast, invalid_app),
        Err(Error::ExpectedFunction {
            expr: invalid_app,
            found: u,
        })
    );
    let invalid_ann = ann(&mut ast, value, linear);
    assert_eq!(
        synthesize(&ast, invalid_ann),
        Err(Error::TypeMismatch {
            expr: value,
            expected: linear,
            found: u,
        })
    );
}

#[test]
fn de_bruijn_levels_must_be_in_scope() {
    let mut ast = Ast::new();
    let free = var(&mut ast, 0);
    assert_eq!(
        synthesize(&ast, free),
        Err(Error::UnboundVariable {
            expr: free,
            level: Level(0),
        })
    );
    let u = ast.unit_type();
    let ty = arrow(&mut ast, Quantity::One, u, u);
    let outside = var(&mut ast, 1);
    let root = lambda(&mut ast, Quantity::One, outside);
    assert_eq!(
        check(&ast, root, ty),
        Err(Error::UnboundVariable {
            expr: outside,
            level: Level(1),
        })
    );
}

#[test]
fn shared_expressions_are_rejected_instead_of_unfolded() {
    let mut ast = Ast::new();
    let u = ast.unit_type();
    let linear = arrow(&mut ast, Quantity::One, u, u);
    let x = var(&mut ast, 0);
    let identity = lambda(&mut ast, Quantity::One, x);
    let shared = ann(&mut ast, identity, linear);
    let value = unit(&mut ast);
    let inner = app(&mut ast, shared, value);
    let root = app(&mut ast, shared, inner);
    assert_eq!(synthesize(&ast, root), Err(Error::SharedExpression(shared)));
}

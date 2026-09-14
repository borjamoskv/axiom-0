use micro_axiom_0::{
    ast::{Ast, Expr, Level, Quantity},
    elaborator::{self, Error},
};

#[test]
fn independently_constructed_equal_types_share_an_id() {
    let mut ast = Ast::new();
    let unit = ast.unit_type();
    let a = ast.function_type(Quantity::One, unit, unit).unwrap();
    let b = ast.function_type(Quantity::One, unit, unit).unwrap();
    assert_eq!(a, b);
    let aa = ast.function_type(Quantity::Omega, a, a).unwrap();
    let bb = ast.function_type(Quantity::Omega, b, b).unwrap();
    assert_eq!(aa, bb);
    let erased = ast.function_type(Quantity::Zero, unit, unit).unwrap();
    assert_ne!(a, erased);
}

#[test]
fn shared_term_occurrences_are_rejected() {
    let mut ast = Ast::new();
    let unit = ast.unit_type();
    let ty = ast.function_type(Quantity::Zero, unit, unit).unwrap();
    let shared = ast.push(Expr::Unit).unwrap();
    let lambda = ast
        .push(Expr::Lambda {
            quantity: Quantity::Zero,
            body: shared,
        })
        .unwrap();
    let function = ast.push(Expr::Ann { term: lambda, ty }).unwrap();
    let root = ast
        .push(Expr::App {
            function,
            argument: shared,
        })
        .unwrap();
    assert_eq!(
        elaborator::synthesize(&ast, root),
        Err(Error::SharedExpression(shared))
    );
}

#[test]
fn a_failed_check_does_not_leak_context_into_the_next_call() {
    let mut ast = Ast::new();
    let unit = ast.unit_type();
    let ty = ast.function_type(Quantity::One, unit, unit).unwrap();
    let variable = ast.push(Expr::Var(Level(1))).unwrap();
    let bad = ast
        .push(Expr::Lambda {
            quantity: Quantity::One,
            body: variable,
        })
        .unwrap();
    assert!(matches!(
        elaborator::check(&ast, bad, ty),
        Err(Error::UnboundVariable { .. })
    ));
    let root = ast.push(Expr::Unit).unwrap();
    assert_eq!(
        elaborator::check(&ast, root, unit).unwrap().visited_nodes,
        1
    );
}

#[test]
fn deep_lambdas_and_types_use_an_explicit_stack() {
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            const DEPTH: usize = 30_000;
            let mut ast = Ast::new();
            let unit = ast.unit_type();
            let mut ty = unit;
            let mut root = ast.push(Expr::Unit).unwrap();
            for _ in 0..DEPTH {
                ty = ast.function_type(Quantity::Omega, unit, ty).unwrap();
                root = ast
                    .push(Expr::Lambda {
                        quantity: Quantity::Omega,
                        body: root,
                    })
                    .unwrap();
            }
            let result = elaborator::check(&ast, root, ty).unwrap();
            assert_eq!(result.ty, ty);
            assert_eq!(result.visited_nodes, DEPTH + 1);
            assert_eq!(result.type_of(root), Some(ty));
            assert!(result.processed_frames() <= 5 * result.visited_nodes);
        })
        .unwrap()
        .join()
        .unwrap();
}

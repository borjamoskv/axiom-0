use micro_axiom_0::{
    ast::{Ast, AstError, Expr, Level, Quantity, Span},
    elaborator::{self, Error},
};
use std::error::Error as _;

#[test]
fn elaboration_records_the_type_of_every_reachable_node_only() {
    let mut ast = Ast::new();
    let u = ast.unit_type();
    let linear = ast.function_type(Quantity::One, u, u).unwrap();
    let unused = ast.push(Expr::Unit).unwrap();
    let variable = ast.push(Expr::Var(Level(0))).unwrap();
    let lambda = ast
        .push(Expr::Lambda {
            quantity: Quantity::One,
            body: variable,
        })
        .unwrap();
    let function = ast
        .push(Expr::Ann {
            term: lambda,
            ty: linear,
        })
        .unwrap();
    let argument = ast.push(Expr::Unit).unwrap();
    let root = ast.push(Expr::App { function, argument }).unwrap();
    let result = elaborator::synthesize(&ast, root).unwrap();
    assert_eq!(result.root(), root);
    assert_eq!(result.ty, u);
    assert_eq!(result.visited_nodes, 5);
    for node in [variable, argument, root] {
        assert_eq!(result.type_of(node), Some(u));
    }
    for node in [lambda, function] {
        assert_eq!(result.type_of(node), Some(linear));
    }
    assert_eq!(result.type_of(unused), None);
    let future = ast.push(Expr::Unit).unwrap();
    assert_eq!(result.type_of(future), None);
    let mut foreign = Ast::new();
    let other = foreign.push(Expr::Unit).unwrap();
    assert_eq!(result.type_of(other), None);
    assert!(result.processed_frames() <= 5 * result.visited_nodes);
}

#[test]
fn checking_a_lambda_records_both_its_type_and_its_body_type() {
    let mut ast = Ast::new();
    let unit = ast.unit_type();
    let function = ast.function_type(Quantity::One, unit, unit).unwrap();
    let x = ast.push(Expr::Var(Level(0))).unwrap();
    let root = ast
        .push(Expr::Lambda {
            quantity: Quantity::One,
            body: x,
        })
        .unwrap();
    let result = elaborator::check(&ast, root, function).unwrap();
    assert_eq!(result.type_of(root), Some(function));
    assert_eq!(result.type_of(x), Some(unit));
}

#[test]
fn diagnostics_locate_the_offending_child_instead_of_the_root() {
    let mut ast = Ast::new();
    let unit = ast.unit_type();
    let identity = ast.function_type(Quantity::One, unit, unit).unwrap();
    let free = ast
        .push_spanned(Expr::Var(Level(1)), Span::new(10, 12).unwrap())
        .unwrap();
    let root = ast
        .push_spanned(
            Expr::Lambda {
                quantity: Quantity::One,
                body: free,
            },
            Span::new(0, 12).unwrap(),
        )
        .unwrap();
    let error = elaborator::check(&ast, root, identity).unwrap_err();
    assert_eq!(error.expression(), Some(free));
    assert_eq!(ast.span(free).unwrap(), Span::new(10, 12));
    let rendered = error.diagnostic(&ast).to_string();
    assert!(rendered.starts_with("bytes 10..12: "));
    assert!(rendered.contains("level 1 is outside its scope"));
}

#[test]
fn diagnostics_do_not_attach_a_foreign_source_range() {
    let mut source = Ast::new();
    let foreign = source.push(Expr::Var(Level(0))).unwrap();
    let mut target = Ast::new();
    target
        .push_spanned(Expr::Unit, Span::new(50, 80).unwrap())
        .unwrap();
    let error = elaborator::synthesize(&target, foreign).unwrap_err();
    assert_eq!(error, Error::Ast(AstError::ForeignExpr(foreign)));
    assert!(error.source().is_some());
    let message = error.diagnostic(&target).to_string();
    assert!(message.contains("belongs to another arena"));
    assert!(!message.contains("bytes"));
}

#[test]
fn usage_diagnostics_include_the_declared_and_observed_quantities() {
    let mut ast = Ast::new();
    let unit = ast.unit_type();
    let linear = ast.function_type(Quantity::One, unit, unit).unwrap();
    let body = ast.push(Expr::Unit).unwrap();
    let root = ast
        .push(Expr::Lambda {
            quantity: Quantity::One,
            body,
        })
        .unwrap();
    let error = elaborator::check(&ast, root, linear).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("quantity 1 does not permit observed usage 0")
    );
    assert_eq!(error.expression(), Some(root));
    assert!(error.source().is_none());
    assert_eq!(error.to_string(), error.diagnostic(&ast).to_string());
}

#[test]
fn source_ranges_are_ordered_and_can_be_empty() {
    assert_eq!(Span::new(5, 4), None);
    let empty = Span::new(5, 5).unwrap();
    assert_eq!((empty.start(), empty.end()), (5, 5));
}

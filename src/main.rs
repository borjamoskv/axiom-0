use micro_axiom_0::ast::{Ast, Expr, Quantity};
use micro_axiom_0::elaborator::check;
use micro_axiom_0::eval::eval;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ast = Ast::new();

    let unit = ast.push(Expr::UnitType)?;
    let identity_ty = ast.push(Expr::Pi { quantity: Quantity::One, domain: unit, codomain: unit })?;

    let body = ast.push(Expr::Var(micro_axiom_0::ast::Level(0)))?;
    let identity = ast.push(Expr::Lambda { quantity: Quantity::One, body })?;

    let identity_val = eval(&ast, identity_ty, &[]);
    let result = check(&ast, identity, identity_val)?;

    println!("Success! Evaluated type: {:?}", result.ty);

    Ok(())
}

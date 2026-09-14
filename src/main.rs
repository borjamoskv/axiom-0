use micro_axiom_0::{
    ast::{Ast, Expr, Level, Quantity},
    elaborator,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke example: ((λ¹ x. x) : Unit →¹ Unit) ().
    let mut ast = Ast::new();
    let unit = ast.unit_type();
    let identity_ty = ast.function_type(Quantity::One, unit, unit)?;
    let variable = ast.push(Expr::Var(Level(0)))?;
    let identity = ast.push(Expr::Lambda {
        quantity: Quantity::One,
        body: variable,
    })?;
    let function = ast.push(Expr::Ann {
        term: identity,
        ty: identity_ty,
    })?;
    let argument = ast.push(Expr::Unit)?;
    let root = ast.push(Expr::App { function, argument })?;
    let result = elaborator::synthesize(&ast, root)?;
    println!(
        "Micro-AXIOM-0: {:?}; nodes={}; frames={}",
        ast.ty(result.ty)?,
        result.visited_nodes,
        result.processed_frames()
    );
    Ok(())
}

with open("src/elaborator.rs", "r") as f:
    content = f.read()

content = content.replace(
    "Expr::Universe | Expr::UnitType => Ok(Elaboration { ty: Value::Universe(0), usages: vec![] }),",
    """Expr::Universe(level) => Ok(Elaboration { ty: Value::Universe(level + 1), usages: vec![] }),
            Expr::UnitType => Ok(Elaboration { ty: Value::Universe(0), usages: vec![] }),"""
)

with open("src/elaborator.rs", "w") as f:
    f.write(content)

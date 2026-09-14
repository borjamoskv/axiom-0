import re

with open("src/elaborator.rs", "r") as f:
    content = f.read()

content = content.replace(
    "Expr::Universe | Expr::UnitType => Ok(Elaboration { ty: Value::Universe, usages: vec![] }),",
    """Expr::Universe(level) => Ok(Elaboration { ty: Value::Universe(level + 1), usages: vec![] }),
            Expr::UnitType => Ok(Elaboration { ty: Value::Universe(0), usages: vec![] }),"""
)

with open("src/elaborator.rs", "w") as f:
    f.write(content)

with open("src/eval.rs", "r") as f:
    eval_content = f.read()
    
eval_content = eval_content.replace(
    "Expr::Universe(level) => Value::Universe(*level),",
    "Expr::Universe(level) => Value::Universe(level),"
)

with open("src/eval.rs", "w") as f:
    f.write(eval_content)

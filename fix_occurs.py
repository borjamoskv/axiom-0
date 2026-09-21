import os

code = """
pub fn occurs(ast: &crate::ast::Ast, turbine: &crate::turbine::TurbineEngine, value: &Value, depth: usize, target: crate::ast::MetaId) -> bool {
    match value {
        Value::Unit | Value::UnitType | Value::Bool | Value::True | Value::False | Value::Universe(_) => false,
        Value::Pair(f, s) => occurs(ast, turbine, f, depth, target) || occurs(ast, turbine, s, depth, target),
        Value::Pi(_, d, cod) | Value::Sigma(_, d, cod) => {
            if occurs(ast, turbine, d, depth, target) { return true; }
            let var = Value::Neutral(Neutral::Var(crate::ast::Level(depth)));
            let cod_val = cod.clone().instantiate(ast, var, Some(turbine));
            occurs(ast, turbine, &cod_val, depth + 1, target)
        }
        Value::Lam(_, body) => {
            let var = Value::Neutral(Neutral::Var(crate::ast::Level(depth)));
            let body_val = body.clone().instantiate(ast, var, Some(turbine));
            occurs(ast, turbine, &body_val, depth + 1, target)
        }
        Value::Neutral(n) => occurs_neu(ast, turbine, n, depth, target),
        Value::Meta(id, spine) => {
            if *id == target { return true; }
            if let Some(solved) = turbine.get_meta(*id) {
                if occurs(ast, turbine, &solved, depth, target) { return true; }
            }
            spine.iter().any(|v| occurs(ast, turbine, v, depth, target))
        }
    }
}

pub fn occurs_neu(ast: &crate::ast::Ast, turbine: &crate::turbine::TurbineEngine, neu: &Neutral, depth: usize, target: crate::ast::MetaId) -> bool {
    match neu {
        Neutral::Var(_) => false,
        Neutral::App(f, a) => occurs_neu(ast, turbine, f, depth, target) || occurs(ast, turbine, a, depth, target),
        Neutral::Fst(n) | Neutral::Snd(n) => occurs_neu(ast, turbine, n, depth, target),
    }
}
"""

with open('src/eval.rs', 'a') as f:
    f.write(code)

use crate::ast::{Ast, Expr, ExprId, Level, Quantity};

#[derive(Clone, Debug)]
pub enum Value {
    Unit,
    UnitType,
    Universe,
    Pi(Quantity, Box<Value>, Closure),
    Lam(Quantity, Closure),
    Neutral(Neutral),
}

#[derive(Clone, Debug)]
pub enum Neutral {
    Var(Level),
    App(Box<Neutral>, Box<Value>),
}

#[derive(Clone, Debug)]
pub struct Closure {
    pub env: Vec<Value>,
    pub body: ExprId,
}

impl Closure {
    pub fn instantiate(self, ast: &Ast, arg: Value) -> Value {
        let mut new_env = self.env;
        new_env.push(arg);
        eval(ast, self.body, &new_env)
    }
}

pub fn eval(ast: &Ast, expr: ExprId, env: &[Value]) -> Value {
    match ast.expr(expr).expect("valid expr") {
        Expr::Var(level) => {
            if let Some(v) = env.get(level.0) { v.clone() } else { Value::Neutral(Neutral::Var(level)) }
        }
        Expr::Unit => Value::Unit,
        Expr::UnitType => Value::UnitType,
        Expr::Universe => Value::Universe,
        Expr::Pi { quantity, domain, codomain } => {
            Value::Pi(quantity, Box::new(eval(ast, domain, env)), Closure { env: env.to_vec(), body: codomain })
        }
        Expr::Lambda { quantity, body } => Value::Lam(quantity, Closure { env: env.to_vec(), body }),
        Expr::App { function, argument } => {
            let f_val = eval(ast, function, env);
            let a_val = eval(ast, argument, env);
            match f_val {
                Value::Lam(_, closure) => closure.instantiate(ast, a_val),
                Value::Neutral(neu) => Value::Neutral(Neutral::App(Box::new(neu), Box::new(a_val))),
                _ => panic!("aplicación mal formada"),
            }
        }
        Expr::Ann { term, .. } => eval(ast, term, env),
    }
}

pub fn equiv(ast: &Ast, a: &Value, b: &Value, depth: usize) -> bool {
    match (a, b) {
        (Value::Unit, Value::Unit) => true,
        (Value::UnitType, Value::UnitType) => true,
        (Value::Universe, Value::Universe) => true,
        (Value::Pi(q1, d1, c1), Value::Pi(q2, d2, c2)) => {
            q1 == q2 && equiv(ast, d1, d2, depth) && {
                let var = Value::Neutral(Neutral::Var(Level(depth)));
                let v1 = c1.clone().instantiate(ast, var.clone());
                let v2 = c2.clone().instantiate(ast, var);
                equiv(ast, &v1, &v2, depth + 1)
            }
        }
        (Value::Lam(q1, c1), Value::Lam(q2, c2)) => {
            q1 == q2 && {
                let var = Value::Neutral(Neutral::Var(Level(depth)));
                let v1 = c1.clone().instantiate(ast, var.clone());
                let v2 = c2.clone().instantiate(ast, var);
                equiv(ast, &v1, &v2, depth + 1)
            }
        }
        (Value::Neutral(n1), Value::Neutral(n2)) => equiv_neu(ast, n1, n2, depth),
        _ => false,
    }
}

fn equiv_neu(ast: &Ast, n1: &Neutral, n2: &Neutral, depth: usize) -> bool {
    match (n1, n2) {
        (Neutral::Var(l1), Neutral::Var(l2)) => l1 == l2,
        (Neutral::App(f1, a1), Neutral::App(f2, a2)) => equiv_neu(ast, f1, f2, depth) && equiv(ast, a1, a2, depth),
        _ => false,
    }
}

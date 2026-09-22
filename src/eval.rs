use crate::ast::{Ast, Expr, ExprId, Level, Quantity, MetaId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Unit,
    NatType,
    Zero,
    Succ(Box<Value>),
    UnitType,
    Universe(u32),
    Pi(crate::ast::Plicity, Quantity, Box<Value>, Closure),
    Lam(crate::ast::Plicity, Quantity, Closure),
    Sigma(Quantity, Box<Value>, Closure),
    Pair(Box<Value>, Box<Value>),
    Bool,
    True,
    False,
    IdType(Box<Value>, Box<Value>, Box<Value>),
    Refl(Box<Value>),
    Meta(MetaId, Vec<Value>),
    Neutral(Neutral),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Neutral {
    Var(Level),
    Ind(Box<Value>, Box<Value>, Box<Value>, Box<Neutral>),
    J(Box<Value>, Box<Value>, Box<Neutral>),
    App(crate::ast::Plicity, Box<Neutral>, Box<Value>),
    Fst(Box<Neutral>),
    Snd(Box<Neutral>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Closure {
    pub env: Vec<Value>,
    pub body: ExprId,
}

impl Closure {
    pub fn instantiate(self, ast: &Ast, arg: Value, turbine: Option<&crate::turbine::TurbineEngine>) -> Value {
        let mut new_env = self.env;
        new_env.push(arg);
        eval(ast, self.body, &new_env, turbine)
    }
}

pub fn eval(ast: &Ast, expr: ExprId, env: &[Value], turbine: Option<&crate::turbine::TurbineEngine>) -> Value {
    match ast.expr(expr).expect("valid expr") {
        Expr::Var(level) => {
            if let Some(v) = env.get(level.0) {
                v.clone()
            } else {
                Value::Neutral(Neutral::Var(level))
            }
        }
        Expr::Unit => Value::Unit,
        Expr::UnitType => Value::UnitType,
        Expr::Bool => Value::Bool,
        Expr::True => Value::True,
        Expr::False => Value::False,
        Expr::If { cond, conseq, alt } => {
            match eval(ast, cond, env, turbine) {
                Value::True => eval(ast, conseq, env, turbine),
                Value::False => eval(ast, alt, env, turbine),
                Value::Neutral(n) => Value::Neutral(Neutral::App(crate::ast::Plicity::Explicit, Box::new(n), Box::new(Value::True))), // Simplified for if
                _ => panic!("if evaluado sobre no-booleano"),
            }
        }
        Expr::Sigma { quantity, domain, codomain } => Value::Sigma(
            quantity,
            Box::new(eval(ast, domain, env, turbine)),
            Closure { env: env.to_vec(), body: codomain }
        ),
        Expr::Pair { first, second } => Value::Pair(
            Box::new(eval(ast, first, env, turbine)),
            Box::new(eval(ast, second, env, turbine))
        ),
        Expr::Fst(body) => match eval(ast, body, env, turbine) {
            Value::Pair(f, _) => *f,
            Value::Neutral(n) => Value::Neutral(Neutral::Fst(Box::new(n))),
            _ => panic!("fst sobre no-par"),
        },
        Expr::Snd(body) => match eval(ast, body, env, turbine) {
            Value::Pair(_, s) => *s,
            Value::Neutral(n) => Value::Neutral(Neutral::Snd(Box::new(n))),
            _ => panic!("snd sobre no-par"),
        },
        Expr::Universe(level) => Value::Universe(level),
        Expr::Pi { plicity, quantity, domain, codomain } => Value::Pi(
            plicity,
            quantity,
            Box::new(eval(ast, domain, env, turbine)),
            Closure { env: env.to_vec(), body: codomain },
        ),
        Expr::Lambda { plicity, quantity, body } => Value::Lam(
            plicity,
            quantity,
            Closure { env: env.to_vec(), body },
        ),
        Expr::App { plicity, function, argument } => {
            let mut f_val = eval(ast, function, env, turbine);
            if let Some(turbine) = turbine {
                let map = turbine.inserted_implicits.read().unwrap();
                if let Some(implicits) = map.get(&expr) {
                    for &imp in implicits {
                        let imp_val = Value::Meta(imp, env.to_vec());
                        f_val = match f_val {
                            Value::Lam(_, _, closure) => closure.instantiate(ast, imp_val, Some(turbine)),
                            Value::Neutral(neu) => Value::Neutral(Neutral::App(crate::ast::Plicity::Implicit, Box::new(neu), Box::new(imp_val))),
                            _ => panic!("Cannot apply implicit to non-lambda"),
                        };
                    }
                }
            }
            let a_val = eval(ast, argument, env, turbine);
            match f_val {
                Value::Lam(_, _, closure) => closure.instantiate(ast, a_val, turbine),
                Value::Neutral(neu) => Value::Neutral(Neutral::App(plicity, Box::new(neu), Box::new(a_val))),
                Value::Meta(id, mut spine) => {
                    spine.push(a_val);
                    Value::Meta(id, spine)
                }
                _ => panic!("aplicación mal formada"),
            }
        }
        Expr::Ann { term, .. } => eval(ast, term, env, turbine),
        Expr::Hole => {
            if let Some(turbine) = turbine {
                let map = turbine.expr_to_meta.read().unwrap();
                if let Some(&meta_id) = map.get(&expr) {
                    Value::Meta(meta_id, Vec::new())
                } else {
                    panic!("Hole encountered in eval but no MetaId was mapped by elaborator!");
                }
            } else {
                panic!("Holes cannot be evaluated directly without elaborator");
            }
        }
        Expr::Meta(id) => Value::Meta(id, Vec::new()),
        Expr::NatType => Value::NatType,
        Expr::Zero => Value::Zero,
        Expr::Succ(n) => Value::Succ(Box::new(eval(ast, n, env, turbine))),
        Expr::Ind { mot, z, s, target } => {
            let mot_val = eval(ast, mot, env, turbine);
            let z_val = eval(ast, z, env, turbine);
            let s_val = eval(ast, s, env, turbine);
            let target_val = eval(ast, target, env, turbine);
            eval_ind(ast, mot_val, z_val, s_val, target_val, turbine)
        }
        Expr::IdType { ty, lhs, rhs } => {
            let ty_val = eval(ast, ty, env, turbine);
            let lhs_val = eval(ast, lhs, env, turbine);
            let rhs_val = eval(ast, rhs, env, turbine);
            Value::IdType(Box::new(ty_val), Box::new(lhs_val), Box::new(rhs_val))
        }
        Expr::Refl(x) => {
            let x_val = eval(ast, x, env, turbine);
            Value::Refl(Box::new(x_val))
        }
        Expr::J { mot, base, target } => {
            let mot_val = eval(ast, mot, env, turbine);
            let base_val = eval(ast, base, env, turbine);
            let target_val = eval(ast, target, env, turbine);
            eval_j(ast, mot_val, base_val, target_val, turbine)
        }
    }
}

pub fn equiv(ast: &Ast, a: &Value, b: &Value, depth: usize) -> bool {
    match (a, b) {
        (Value::Unit, Value::Unit) => true,
        (Value::UnitType, Value::UnitType) => true,
        (Value::NatType, Value::NatType) => true,
        (Value::Zero, Value::Zero) => true,
        (Value::Succ(n1), Value::Succ(n2)) => equiv(ast, n1, n2, depth),
        (Value::IdType(t1, l1, r1), Value::IdType(t2, l2, r2)) => {
            equiv(ast, t1, t2, depth) && equiv(ast, l1, l2, depth) && equiv(ast, r1, r2, depth)
        }
        (Value::Refl(x1), Value::Refl(x2)) => equiv(ast, x1, x2, depth),
        (Value::Universe(l1), Value::Universe(l2)) => l1 == l2,
        (Value::Pi(p1, q1, d1, c1), Value::Pi(p2, q2, d2, c2)) => {
            if p1 != p2 { return false; }
            q1 == q2 && equiv(ast, d1, d2, depth) && {
                let var = Value::Neutral(Neutral::Var(Level(depth)));
                let v1 = c1.clone().instantiate(ast, var.clone(), None);
                let v2 = c2.clone().instantiate(ast, var, None);
                equiv(ast, &v1, &v2, depth + 1)
            }
        }
        (Value::Lam(p1, q1, c1), Value::Lam(p2, q2, c2)) => {
            if p1 != p2 { return false; }
            q1 == q2 && {
                let var = Value::Neutral(Neutral::Var(Level(depth)));
                let v1 = c1.clone().instantiate(ast, var.clone(), None);
                let v2 = c2.clone().instantiate(ast, var, None);
                equiv(ast, &v1, &v2, depth + 1)
            }
        }
        (Value::Lam(_p, _q, c), Value::Neutral(n)) => {
            let var = Value::Neutral(Neutral::Var(Level(depth)));
            let v1 = c.clone().instantiate(ast, var.clone(), None);
            let v2 = Value::Neutral(Neutral::App(crate::ast::Plicity::Explicit, Box::new(n.clone()), Box::new(var)));
            equiv(ast, &v1, &v2, depth + 1)
        }
        (Value::Neutral(n), Value::Lam(_p, _q, c)) => {
            let var = Value::Neutral(Neutral::Var(Level(depth)));
            let v1 = Value::Neutral(Neutral::App(crate::ast::Plicity::Explicit, Box::new(n.clone()), Box::new(var.clone())));
            let v2 = c.clone().instantiate(ast, var, None);
            equiv(ast, &v1, &v2, depth + 1)
        }
        (Value::Bool, Value::Bool) => true,
        (Value::True, Value::True) => true,
        (Value::False, Value::False) => true,
        (Value::Pair(f1, s1), Value::Pair(f2, s2)) => equiv(ast, f1, f2, depth) && equiv(ast, s1, s2, depth),
        (Value::Sigma(q1, d1, c1), Value::Sigma(q2, d2, c2)) => {
            q1 == q2 && equiv(ast, d1, d2, depth) && {
                let var = Value::Neutral(Neutral::Var(Level(depth)));
                let v1 = c1.clone().instantiate(ast, var.clone(), None);
                let v2 = c2.clone().instantiate(ast, var, None);
                equiv(ast, &v1, &v2, depth + 1)
            }
        }
        (Value::Neutral(n1), Value::Neutral(n2)) => equiv_neu(ast, n1, n2, depth),
        (Value::Meta(m1, sp1), Value::Meta(m2, sp2)) => {
            m1 == m2 && sp1.len() == sp2.len() && sp1.iter().zip(sp2.iter()).all(|(a, b)| equiv(ast, a, b, depth))
        }
        _ => false,
    }
}

pub fn equiv_neu(ast: &Ast, n1: &Neutral, n2: &Neutral, depth: usize) -> bool {
    match (n1, n2) {
        (Neutral::Var(l1), Neutral::Var(l2)) => l1 == l2,
        (Neutral::App(p1, f1, a1), Neutral::App(p2, f2, a2)) => {
            if p1 != p2 { return false; }
            equiv_neu(ast, f1, f2, depth) && equiv(ast, a1, a2, depth)
        }
        (Neutral::Fst(n1), Neutral::Fst(n2)) => equiv_neu(ast, n1, n2, depth),
        (Neutral::Snd(n1), Neutral::Snd(n2)) => equiv_neu(ast, n1, n2, depth),
        (Neutral::Ind(m1, z1, s1, t1), Neutral::Ind(m2, z2, s2, t2)) => {
            equiv(ast, m1, m2, depth)
                && equiv(ast, z1, z2, depth)
                && equiv(ast, s1, s2, depth)
                && equiv_neu(ast, t1, t2, depth)
        }
        (Neutral::J(m1, b1, t1), Neutral::J(m2, b2, t2)) => {
            equiv(ast, m1, m2, depth)
                && equiv(ast, b1, b2, depth)
                && equiv_neu(ast, t1, t2, depth)
        }
        _ => false,
    }
}

pub fn apply(ast: &Ast, plicity: crate::ast::Plicity, function: Value, argument: Value, turbine: Option<&crate::turbine::TurbineEngine>) -> Value {
    match function {
        Value::Lam(_, _, closure) => closure.instantiate(ast, argument, turbine),
        Value::Neutral(neu) => Value::Neutral(Neutral::App(plicity, Box::new(neu), Box::new(argument))),
        Value::Meta(id, mut spine) => {
            spine.push(argument);
            Value::Meta(id, spine)
        }
        _ => panic!("Cannot apply to non-function"),
    }
}

fn eval_ind(ast: &Ast, mot: Value, z: Value, s: Value, target: Value, turbine: Option<&crate::turbine::TurbineEngine>) -> Value {
    match target {
        Value::Zero => z,
        Value::Succ(n) => {
            let ih = eval_ind(ast, mot.clone(), z, s.clone(), *n.clone(), turbine);
            let step_applied = apply(ast, crate::ast::Plicity::Explicit, s, *n, turbine);
            apply(ast, crate::ast::Plicity::Explicit, step_applied, ih, turbine)
        }
        Value::Neutral(n) => Value::Neutral(Neutral::Ind(Box::new(mot), Box::new(z), Box::new(s), Box::new(n))),
        _ => panic!("Invalid target for ind"),
    }
}

fn eval_j(_ast: &Ast, mot: Value, base: Value, target: Value, _turbine: Option<&crate::turbine::TurbineEngine>) -> Value {
    match target {
        Value::Refl(_) => base,
        Value::Neutral(n) => Value::Neutral(Neutral::J(Box::new(mot), Box::new(base), Box::new(n))),
        _ => panic!("Invalid target for J: must be Refl or Neutral"),
    }
}

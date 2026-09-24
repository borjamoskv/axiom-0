import re

# Fix AST match
with open('src/ast.rs', 'r') as f:
    c = f.read()

c = c.replace('Expr::Succ(n) => self.expr(n)?,', 'Expr::Succ(n) => {\n                self.expr(n)?;\n            }')
with open('src/ast.rs', 'w') as f:
    f.write(c)

# Eval
with open('src/eval.rs', 'r') as f:
    c = f.read()

c = c.replace('pub enum Value {\n    Unit,', 'pub enum Value {\n    Unit,\n    NatType,\n    Zero,\n    Succ(Box<Value>),')
c = c.replace('pub enum Neutral {\n    Var(Level),', 'pub enum Neutral {\n    Var(Level),\n    Ind(Box<Value>, Box<Value>, Box<Value>, Box<Neutral>),')

match_eval_app = """        Expr::App { plicity, function, argument } => {
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
                _ => panic!("Cannot apply to non-function"),
            }
        }"""

repl_eval_app = """        Expr::App { plicity, function, argument } => {
            let mut f_val = eval(ast, function, env, turbine);
            if let Some(turbine) = turbine {
                let map = turbine.inserted_implicits.read().unwrap();
                if let Some(implicits) = map.get(&expr) {
                    for &imp in implicits {
                        let imp_val = Value::Meta(imp, env.to_vec());
                        f_val = apply(ast, crate::ast::Plicity::Implicit, f_val, imp_val, turbine);
                    }
                }
            }
            let a_val = eval(ast, argument, env, turbine);
            apply(ast, plicity, f_val, a_val, turbine)
        }
        Expr::NatType => Value::NatType,
        Expr::Zero => Value::Zero,
        Expr::Succ(n) => Value::Succ(Box::new(eval(ast, n, env, turbine))),
        Expr::Ind { mot, z, s, target } => {
            let mot_val = eval(ast, mot, env, turbine);
            let z_val = eval(ast, z, env, turbine);
            let s_val = eval(ast, s, env, turbine);
            let target_val = eval(ast, target, env, turbine);
            eval_ind(ast, mot_val, z_val, s_val, target_val, turbine)
        }"""

c = c.replace(match_eval_app, repl_eval_app)

match_quote = """        Value::True => ast.push(Expr::True).unwrap(),
        Value::False => ast.push(Expr::False).unwrap(),"""
repl_quote = """        Value::True => ast.push(Expr::True).unwrap(),
        Value::False => ast.push(Expr::False).unwrap(),
        Value::NatType => ast.push(Expr::NatType).unwrap(),
        Value::Zero => ast.push(Expr::Zero).unwrap(),
        Value::Succ(n) => {
            let n_q = quote(ast, *n, levels);
            ast.push(Expr::Succ(n_q)).unwrap()
        }"""
c = c.replace(match_quote, repl_quote)

match_quote_neu = """        Neutral::Snd(neu) => {
            let n = quote_neutral(ast, *neu, levels);
            ast.push(Expr::Snd(n)).unwrap()
        }
    }
}"""
repl_quote_neu = """        Neutral::Snd(neu) => {
            let n = quote_neutral(ast, *neu, levels);
            ast.push(Expr::Snd(n)).unwrap()
        }
        Neutral::Ind(mot, z, s, target) => {
            let mot = quote(ast, *mot, levels);
            let z = quote(ast, *z, levels);
            let s = quote(ast, *s, levels);
            let target = quote_neutral(ast, *target, levels);
            ast.push(Expr::Ind { mot, z, s, target }).unwrap()
        }
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
}"""

c = c.replace(match_quote_neu, repl_quote_neu)

with open('src/eval.rs', 'w') as f:
    f.write(c)


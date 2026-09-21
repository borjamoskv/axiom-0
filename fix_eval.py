import re

with open('src/eval.rs', 'r') as f:
    c = f.read()

c = c.replace('Pi(Quantity, Box<Value>, Closure),', 'Pi(crate::ast::Plicity, Quantity, Box<Value>, Closure),')
c = c.replace('Lam(Quantity, Closure),', 'Lam(crate::ast::Plicity, Quantity, Closure),')
c = c.replace('App(Box<Neutral>, Box<Value>),', 'App(crate::ast::Plicity, Box<Neutral>, Box<Value>),')

# Expr::Pi mapping
c = c.replace('Expr::Pi { quantity, domain, codomain } => Value::Pi(quantity, Box::new(eval(ast, domain, env, turbine)),', 'Expr::Pi { plicity, quantity, domain, codomain } => Value::Pi(plicity, quantity, Box::new(eval(ast, domain, env, turbine)),')
# Expr::Lambda mapping
c = c.replace('Expr::Lambda { quantity, body } => Value::Lam(quantity, Closure { env: env.to_vec(), body },', 'Expr::Lambda { plicity, quantity, body } => Value::Lam(plicity, quantity, Closure { env: env.to_vec(), body },')

# Expr::App mapping with inserted implicits
match_app = """        Expr::App { function, argument } => {
            let f_val = eval(ast, function, env, turbine);
            let a_val = eval(ast, argument, env, turbine);
            match f_val {
                Value::Lam(_, closure) => closure.instantiate(ast, a_val, turbine),
                Value::Neutral(neu) => Value::Neutral(Neutral::App(Box::new(neu), Box::new(a_val))),
                Value::Meta(id, mut spine) => {
                    spine.push(a_val);
                    Value::Meta(id, spine)
                }
                _ => panic!("aplicación mal formada"),
            }
        }"""

repl_app = """        Expr::App { plicity, function, argument } => {
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
        }"""
c = c.replace(match_app, repl_app)

# Equality checks
c = c.replace('        (Value::Pi(q1, d1, c1), Value::Pi(q2, d2, c2)) => {', '        (Value::Pi(p1, q1, d1, c1), Value::Pi(p2, q2, d2, c2)) => {\n            if p1 != p2 { return false; }')
c = c.replace('        (Value::Lam(q1, c1), Value::Lam(q2, c2)) => {', '        (Value::Lam(p1, q1, c1), Value::Lam(p2, q2, c2)) => {\n            if p1 != p2 { return false; }')
c = c.replace('        (Neutral::App(f1, a1), Neutral::App(f2, a2)) => {', '        (Neutral::App(p1, f1, a1), Neutral::App(p2, f2, a2)) => {\n            if p1 != p2 { return false; }')

with open('src/eval.rs', 'w') as f:
    f.write(c)

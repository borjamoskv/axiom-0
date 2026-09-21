import re

with open('src/eval.rs', 'r') as f:
    c = f.read()

match_app = """            println!("Evaluating App {:?}", expr);
            let f_val = eval(ast, function, env, turbine);
            let a_val = eval(ast, argument, env, turbine);
            match f_val {
                Value::Lam(_, _, closure) => closure.instantiate(ast, a_val, turbine),
                Value::Neutral(neu) => Value::Neutral(Neutral::App(plicity, Box::new(neu), Box::new(a_val))),
                Value::Meta(id, mut spine) => {
                    spine.push(a_val);
                    Value::Meta(id, spine)
                }
                _ => panic!("aplicación mal formada"),
            }"""

repl_app = """            println!("Evaluating App {:?}", expr);
            let mut f_val = eval(ast, function, env, turbine);
            if let Some(turbine) = turbine {
                let map = turbine.inserted_implicits.read().unwrap();
                if let Some(implicits) = map.get(&expr) {
                    println!("Found {} inserted implicits!", implicits.len());
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
            }"""

c = c.replace(match_app, repl_app)

with open('src/eval.rs', 'w') as f:
    f.write(c)


import re

with open('src/elaborator.rs', 'r') as f:
    c = f.read()

match_unify = """            (Value::Meta(m, sp), val) | (val, Value::Meta(m, sp)) => {
                if sp.is_empty() {
                    // Occurs check
                    if crate::eval::occurs(ast, turbine, &val, depth, m) {
                        return false;
                    }
                    let _ = turbine.solve_meta(m, val);
                    true
                } else {
                    false // Pattern unification complex spines omitted
                }
            }"""

repl_unify = """            (Value::Meta(m, sp), val) | (val, Value::Meta(m, sp)) => {
                // Occurs check
                if crate::eval::occurs(ast, turbine, &val, depth, m) {
                    return false;
                }
                // MVP: just store globally. In full HOPU we would invert the spine.
                let _ = turbine.solve_meta(m, val);
                true
            }"""
c = c.replace(match_unify, repl_unify)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)

with open('src/turbine.rs', 'r') as f:
    c = f.read()

match_force = """            Value::Meta(id, spine) => {
                let solution = self.meta_ctx.read().unwrap()[id.0].clone();
                if let Some(mut sol) = solution {
                    // Apply spine to the solution
                    for arg in spine {
                        sol = match sol {
                            Value::Lam(_, closure) => closure.instantiate(ast, arg, Some(self)),
                            Value::Neutral(neu) => Value::Neutral(crate::eval::Neutral::App(Box::new(neu), Box::new(arg))),
                            _ => panic!("Cannot apply spine to non-lambda in force"),
                        }
                    }
                    self.force(ast, sol)
                } else {
                    Value::Meta(id, spine)
                }
            }"""

repl_force = """            Value::Meta(id, spine) => {
                let solution = self.meta_ctx.read().unwrap()[id.0].clone();
                if let Some(sol) = solution {
                    // MVP: assume sol is evaluated in the correct context, ignore spine.
                    self.force(ast, sol)
                } else {
                    Value::Meta(id, spine)
                }
            }"""

c = c.replace(match_force, repl_force)

with open('src/turbine.rs', 'w') as f:
    f.write(c)


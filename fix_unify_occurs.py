import re

with open('src/elaborator.rs', 'r') as f:
    c = f.read()

match_unify = """            (Value::Meta(m, sp), val) | (val, Value::Meta(m, sp)) => {
                if sp.is_empty() {
                    // Occurs check omitted for simplicity in this MVP
                    turbine.solve_meta(m, val);
                    true
                } else {
                    false // Pattern unification complex spines omitted
                }
            }"""

repl_unify = """            (Value::Meta(m, sp), val) | (val, Value::Meta(m, sp)) => {
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

c = c.replace(match_unify, repl_unify)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)

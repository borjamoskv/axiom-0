import re

with open('src/eval.rs', 'r') as f:
    c = f.read()

match_eval_imp = """                let map = turbine.inserted_implicits.read().unwrap();
                if let Some(implicits) = map.get(&expr) {
                    for &imp in implicits {"""

repl_eval_imp = """                let map = turbine.inserted_implicits.read().unwrap();
                if let Some(implicits) = map.get(&expr) {
                    println!("Found {} implicits for app {}!", implicits.len(), expr.index);
                    for &imp in implicits {"""

c = c.replace(match_eval_imp, repl_eval_imp)

with open('src/eval.rs', 'w') as f:
    f.write(c)


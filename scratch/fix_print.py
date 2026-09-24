import re

with open('src/eval.rs', 'r') as f:
    c = f.read()

match_eval = """                if let Some(implicits) = map.get(&expr) {
                    for &imp in implicits {"""

repl_eval = """                if let Some(implicits) = map.get(&expr) {
                    println!("Found {} implicits for app!", implicits.len());
                    for &imp in implicits {"""

c = c.replace(match_eval, repl_eval)

with open('src/eval.rs', 'w') as f:
    f.write(c)


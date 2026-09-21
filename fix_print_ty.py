import re

with open('src/elaborator.rs', 'r') as f:
    c = f.read()

match_elab_while = """                // Implicit insertion loop
                let mut implicits_to_insert = Vec::new();
                while let Value::Pi(plic, _decl_q, _dom, cod) = f_elab.ty.clone() {"""

repl_elab_while = """                // Implicit insertion loop
                let mut implicits_to_insert = Vec::new();
                println!("f_elab.ty is {:?}", f_elab.ty);
                println!("App plicity is {:?}", plicity);
                while let Value::Pi(plic, _decl_q, _dom, cod) = f_elab.ty.clone() {"""

c = c.replace(match_elab_while, repl_elab_while)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)


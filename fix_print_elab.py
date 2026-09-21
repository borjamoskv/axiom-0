import re

with open('src/elaborator.rs', 'r') as f:
    c = f.read()

match_elab_app = """            Expr::App { plicity, function, argument } => {
                let mut f_elab = self.synth(function, depth, env, types)?;

                let mut implicits_to_insert = Vec::new();"""

repl_elab_app = """            Expr::App { plicity, function, argument } => {
                let mut f_elab = self.synth(function, depth, env, types)?;
                println!("Synthesized function type: {:?}", f_elab.ty);
                println!("Application plicity: {:?}", plicity);

                let mut implicits_to_insert = Vec::new();"""

c = c.replace(match_elab_app, repl_elab_app)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)


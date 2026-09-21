import re

with open('src/elaborator.rs', 'r') as f:
    c = f.read()

match_app = """            Expr::App { plicity, function, argument } => {
                let mut f_elab = self.synth(function, depth, env, types)?;
                if let Value::Pi(plic, decl_q, dom, cod) = f_elab.ty {"""

repl_app = """            Expr::App { plicity, function, argument } => {
                let mut f_elab = self.synth(function, depth, env, types)?;
                
                let mut implicits_to_insert = Vec::new();
                while let Value::Pi(plic, _decl_q, _dom, cod) = f_elab.ty.clone() {
                    if plic == crate::ast::Plicity::Implicit && plicity == crate::ast::Plicity::Explicit {
                        if let Some(turbine) = self.turbine {
                            let meta_val_id = turbine.new_meta();
                            let meta_val = Value::Meta(meta_val_id, env.to_vec());
                            f_elab.ty = cod.instantiate(self.ast, meta_val, self.turbine);
                            implicits_to_insert.push(meta_val_id);
                        } else {
                            return Err(Error::TypeMismatch {
                                expr: function,
                                expected: "Explicit function".to_string(),
                                found: "Implicit function without Turbine".to_string(),
                            });
                        }
                    } else {
                        break;
                    }
                }

                if !implicits_to_insert.is_empty() {
                    if let Some(turbine) = self.turbine {
                        turbine.inserted_implicits.write().unwrap().insert(expr, implicits_to_insert);
                    }
                }

                if let Value::Pi(plic, decl_q, dom, cod) = f_elab.ty {"""

c = c.replace(match_app, repl_app)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)

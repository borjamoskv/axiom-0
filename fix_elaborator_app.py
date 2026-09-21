import re

with open('src/elaborator.rs', 'r') as f:
    c = f.read()

match_app = """            Expr::App { plicity, function, argument } => {
                let mut f_elab = self.synth(function, depth, env, types)?;
                if let Value::Pi(plic, decl_q, dom, cod) = f_elab.ty {
                    let mut arg_elab = self.check(argument, *dom, depth, env, types)?;"""

repl_app = """            Expr::App { plicity, function, argument } => {
                let mut f_elab = self.synth(function, depth, env, types)?;

                // Implicit insertion loop
                while let Value::Pi(plic, decl_q, dom, cod) = f_elab.ty.clone() {
                    if plic == crate::ast::Plicity::Implicit && plicity == crate::ast::Plicity::Explicit {
                        if let Some(turbine) = self.turbine {
                            let meta_val_id = turbine.new_meta();
                            let meta_val = Value::Meta(meta_val_id, env.to_vec());
                            
                            // Unify the hole with the domain (it's just a placeholder)
                            f_elab.ty = cod.instantiate(self.ast, meta_val, self.turbine);
                            // We might also need to insert it in the AST if we were mutating the AST,
                            // but in C5-REAL we don't rewrite the AST.
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

                if let Value::Pi(plic, decl_q, dom, cod) = f_elab.ty {
                    if plic != plicity {
                        return Err(Error::TypeMismatch {
                            expr: argument,
                            expected: format!("{:?} argument", plic),
                            found: format!("{:?} application", plicity),
                        });
                    }

                    let mut arg_elab = self.check(argument, *dom, depth, env, types)?;"""

c = c.replace(match_app, repl_app)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)


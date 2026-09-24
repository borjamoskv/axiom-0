import re

with open('src/elaborator.rs', 'r') as f:
    c = f.read()

# Update Pi checking
c = c.replace('            Expr::Pi { quantity, domain, codomain } => {', '            Expr::Pi { plicity: _, quantity, domain, codomain } => {')

# Update Lam synthesis
c = c.replace('            Expr::Lambda { quantity, body } => {', '            Expr::Lambda { plicity, quantity, body } => {')
c = c.replace('                let ty = Value::Pi(*quantity, Box::new(domain_ty), Closure {', '                let ty = Value::Pi(*plicity, *quantity, Box::new(domain_ty), Closure {')

# Update App synthesis with Implicit Insertion
match_app = """            Expr::App { function, argument } => {
                let f_elab = self.synth(function, depth, env, types)?;
                
                if let Value::Pi(_, dom, cod) = f_elab.ty {
                    let mut arg_elab = self.check(argument, *dom, depth, env, types)?;"""

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

                if let Value::Pi(plic, _, dom, cod) = f_elab.ty {
                    if plic != plicity {
                        return Err(Error::TypeMismatch {
                            expr: argument,
                            expected: format!("{:?} argument", plic),
                            found: format!("{:?} application", plicity),
                        });
                    }

                    let mut arg_elab = self.check(argument, *dom, depth, env, types)?;"""

c = c.replace(match_app, repl_app)

# Replace remaining Pi pattern matches in elaborator.rs
c = c.replace('Value::Pi(_, dom, cod)', 'Value::Pi(_, _, dom, cod)')
c = c.replace('Value::Pi(_, dom1, cod1)', 'Value::Pi(_, _, dom1, cod1)')
c = c.replace('Value::Pi(_, dom2, cod2)', 'Value::Pi(_, _, dom2, cod2)')
c = c.replace('Expr::Lambda { quantity: _, body }', 'Expr::Lambda { plicity: _, quantity: _, body }')

with open('src/elaborator.rs', 'w') as f:
    f.write(c)


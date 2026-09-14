import re

with open("src/elaborator.rs", "r") as f:
    content = f.read()

# Replace Universe matching in synth_term
content = re.sub(
    r"Expr::Universe => Ok\(Elaboration \{ ty: Value::Universe, usages: vec!\[\] \}\),",
    r"Expr::Universe(level) => Ok(Elaboration { ty: Value::Universe(level + 1), usages: vec![] }),",
    content
)

content = re.sub(
    r"Expr::UnitType => Ok\(Elaboration \{ ty: Value::Universe, usages: vec!\[\] \}\),",
    r"Expr::UnitType => Ok(Elaboration { ty: Value::Universe(0), usages: vec![] }),",
    content
)

# Replace Universe in Pi
pi_old = """            Expr::Pi { quantity: _, domain, codomain } => {
                // A type annotation uses variables at quantity ZERO because it is erased at runtime.
                // We check them, but we multiply their usage by ZERO (or just discard).
                self.check(domain, Value::Universe, depth, env, types)?;
                let var = Value::Neutral(crate::eval::Neutral::Var(Level(depth)));
                let mut new_env = env.to_vec();
                new_env.push(var);
                let dom_val = eval(self.ast, domain, env);
                let mut new_types = types.to_vec();
                new_types.push(dom_val);
                self.check(codomain, Value::Universe, depth + 1, &new_env, &new_types)?;
                Ok(Elaboration { ty: Value::Universe, usages: vec![] })
            }"""

pi_new = """            Expr::Pi { quantity: _, domain, codomain } => {
                let dom_elab = self.synth(domain, depth, env, types)?;
                let u_dom = match dom_elab.ty {
                    Value::Universe(u) => u,
                    _ => return Err(Error::TypeMismatch { expr: domain, expected: "Universe".into(), found: format!("{:?}", dom_elab.ty) }),
                };
                let var = Value::Neutral(crate::eval::Neutral::Var(Level(depth)));
                let mut new_env = env.to_vec();
                new_env.push(var);
                let dom_val = eval(self.ast, domain, env);
                let mut new_types = types.to_vec();
                new_types.push(dom_val);
                let cod_elab = self.synth(codomain, depth + 1, &new_env, &new_types)?;
                let u_cod = match cod_elab.ty {
                    Value::Universe(u) => u,
                    _ => return Err(Error::TypeMismatch { expr: codomain, expected: "Universe".into(), found: format!("{:?}", cod_elab.ty) }),
                };
                Ok(Elaboration { ty: Value::Universe(u_dom.max(u_cod)), usages: vec![] })
            }"""

content = content.replace(pi_old, pi_new)

ann_old = """            Expr::Ann { term, ty } => {
                self.check(ty, Value::Universe, depth, env, types)?;
                let ty_val = eval(self.ast, ty, env);
                let elab = self.check(term, ty_val.clone(), depth, env, types)?;
                Ok(elab)
            }"""

ann_new = """            Expr::Ann { term, ty } => {
                let ty_elab = self.synth(ty, depth, env, types)?;
                match ty_elab.ty {
                    Value::Universe(_) => {},
                    _ => return Err(Error::TypeMismatch { expr: ty, expected: "Universe".into(), found: format!("{:?}", ty_elab.ty) }),
                };
                let ty_val = eval(self.ast, ty, env);
                let elab = self.check(term, ty_val.clone(), depth, env, types)?;
                Ok(elab)
            }"""

content = content.replace(ann_old, ann_new)

# Also fix the Pi expected type in check:
# if let Value::Pi(decl_q, dom, cod_closure) = expected
# wait, what if Expected is Universe?
# check function currently doesn't synthesize if expected is Universe.
# It delegates to synth_term. That's fine.

with open("src/elaborator.rs", "w") as f:
    f.write(content)

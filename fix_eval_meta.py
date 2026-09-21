import re

with open('src/turbine.rs', 'r') as f:
    c = f.read()

c = c.replace('pub inserted_implicits: RwLock<std::collections::HashMap<crate::ast::ExprId, Vec<crate::ast::ExprId>>>', 'pub inserted_implicits: RwLock<std::collections::HashMap<crate::ast::ExprId, Vec<crate::ast::MetaId>>>')

with open('src/turbine.rs', 'w') as f:
    f.write(c)

with open('src/eval.rs', 'r') as f:
    c = f.read()

match_eval_imp = """                let map = turbine.inserted_implicits.read().unwrap();
                if let Some(implicits) = map.get(&expr) {
                    for &imp in implicits {
                        let imp_val = eval(ast, imp, env, Some(turbine));
                        f = match f {
                            Value::Lam(_, _, closure) => closure.instantiate(ast, imp_val, Some(turbine)),
                            Value::Neutral(neu) => Value::Neutral(Neutral::App(crate::ast::Plicity::Implicit, Box::new(neu), Box::new(imp_val))),
                            _ => panic!("Cannot apply implicit to non-lambda"),
                        };
                    }
                }"""

repl_eval_imp = """                let map = turbine.inserted_implicits.read().unwrap();
                if let Some(implicits) = map.get(&expr) {
                    for &imp in implicits {
                        let imp_val = Value::Meta(imp, env.to_vec());
                        f = match f {
                            Value::Lam(_, _, closure) => closure.instantiate(ast, imp_val, Some(turbine)),
                            Value::Neutral(neu) => Value::Neutral(Neutral::App(crate::ast::Plicity::Implicit, Box::new(neu), Box::new(imp_val))),
                            _ => panic!("Cannot apply implicit to non-lambda"),
                        };
                    }
                }"""

c = c.replace(match_eval_imp, repl_eval_imp)

with open('src/eval.rs', 'w') as f:
    f.write(c)

with open('src/elaborator.rs', 'r') as f:
    c = f.read()

match_elab = """                let mut implicits_to_insert = Vec::new();
                while let Value::Pi(plic, _decl_q, _dom, cod) = f_elab.ty.clone() {
                    if plic == crate::ast::Plicity::Implicit && plicity == crate::ast::Plicity::Explicit {
                        if let Some(turbine) = self.turbine {
                            let hole_expr = self.ast.push(Expr::Hole).unwrap();
                            let meta_val_id = turbine.new_meta();
                            turbine.expr_to_meta.write().unwrap().insert(hole_expr, meta_val_id);
                            
                            let meta_val = Value::Meta(meta_val_id, env.to_vec());
                            f_elab.ty = cod.instantiate(self.ast, meta_val, self.turbine);
                            implicits_to_insert.push(hole_expr);
                        } else {"""

repl_elab = """                let mut implicits_to_insert = Vec::new();
                while let Value::Pi(plic, _decl_q, _dom, cod) = f_elab.ty.clone() {
                    if plic == crate::ast::Plicity::Implicit && plicity == crate::ast::Plicity::Explicit {
                        if let Some(turbine) = self.turbine {
                            let meta_val_id = turbine.new_meta();
                            let meta_val = Value::Meta(meta_val_id, env.to_vec());
                            f_elab.ty = cod.instantiate(self.ast, meta_val, self.turbine);
                            implicits_to_insert.push(meta_val_id);
                        } else {"""

c = c.replace(match_elab, repl_elab)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)


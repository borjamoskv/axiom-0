import re

with open('src/turbine.rs', 'r') as f:
    c = f.read()

# Add inserted_implicits to TurbineEngine
match_struct = """pub struct TurbineEngine {
    pub ast: crate::ast::Ast,
    pub meta_ctx: RwLock<Vec<Option<Value>>>,
    pub expr_to_meta: RwLock<HashMap<ExprId, MetaId>>,
}"""

repl_struct = """pub struct TurbineEngine {
    pub ast: crate::ast::Ast,
    pub meta_ctx: RwLock<Vec<Option<Value>>>,
    pub expr_to_meta: RwLock<HashMap<ExprId, MetaId>>,
    pub inserted_implicits: RwLock<HashMap<ExprId, Vec<ExprId>>>,
}"""

c = c.replace(match_struct, repl_struct)

# Add initialization
match_init = """            meta_ctx: RwLock::new(Vec::new()),
            expr_to_meta: RwLock::new(HashMap::new()),
        }"""

repl_init = """            meta_ctx: RwLock::new(Vec::new()),
            expr_to_meta: RwLock::new(HashMap::new()),
            inserted_implicits: RwLock::new(HashMap::new()),
        }"""

c = c.replace(match_init, repl_init)

with open('src/turbine.rs', 'w') as f:
    f.write(c)

with open('src/eval.rs', 'r') as f:
    c = f.read()

# Modify eval to check inserted_implicits
match_eval = """        Expr::App { plicity, function, argument } => {
            let f = eval(ast, function, env, turbine);
            let a = eval(ast, argument, env, turbine);"""

repl_eval = """        Expr::App { plicity, function, argument } => {
            let mut f = eval(ast, function, env, turbine);
            if let Some(turbine) = turbine {
                let map = turbine.inserted_implicits.read().unwrap();
                if let Some(implicits) = map.get(&expr) {
                    for &imp in implicits {
                        let imp_val = eval(ast, imp, env, Some(turbine));
                        f = match f {
                            Value::Lam(_, _, closure) => closure.instantiate(ast, imp_val, Some(turbine)),
                            Value::Neutral(neu) => Value::Neutral(Neutral::App(crate::ast::Plicity::Implicit, Box::new(neu), Box::new(imp_val))),
                            _ => panic!("Cannot apply implicit to non-lambda"),
                        };
                    }
                }
            }
            let a = eval(ast, argument, env, turbine);"""

c = c.replace(match_eval, repl_eval)

with open('src/eval.rs', 'w') as f:
    f.write(c)

with open('src/elaborator.rs', 'r') as f:
    c = f.read()

match_elab = """                // Implicit insertion loop
                while let Value::Pi(plic, _decl_q, _dom, cod) = f_elab.ty.clone() {
                    if plic == crate::ast::Plicity::Implicit && plicity == crate::ast::Plicity::Explicit {
                        if let Some(turbine) = self.turbine {
                            let meta_val_id = turbine.new_meta();
                            let meta_val = Value::Meta(meta_val_id, env.to_vec());
                            
                            // Unify the hole with the domain (it's just a placeholder)
                            f_elab.ty = cod.instantiate(self.ast, meta_val, self.turbine);
                            // We might also need to insert it in the AST if we were mutating the AST,
                            // but in C5-REAL we don't rewrite the AST.
                        } else {"""

repl_elab = """                // Implicit insertion loop
                let mut implicits_to_insert = Vec::new();
                while let Value::Pi(plic, _decl_q, _dom, cod) = f_elab.ty.clone() {
                    if plic == crate::ast::Plicity::Implicit && plicity == crate::ast::Plicity::Explicit {
                        if let Some(turbine) = self.turbine {
                            // Synthesize a hole
                            let hole_expr = self.ast.push(Expr::Hole).unwrap();
                            let meta_val_id = turbine.new_meta();
                            turbine.expr_to_meta.write().unwrap().insert(hole_expr, meta_val_id);
                            
                            let meta_val = Value::Meta(meta_val_id, env.to_vec());
                            f_elab.ty = cod.instantiate(self.ast, meta_val, self.turbine);
                            implicits_to_insert.push(hole_expr);
                        } else {"""

c = c.replace(match_elab, repl_elab)

match_elab_end = """                    let mut arg_elab = self.check(argument, *_dom, depth, env, types)?;"""

repl_elab_end = """                    if !implicits_to_insert.is_empty() {
                        if let Some(turbine) = self.turbine {
                            turbine.inserted_implicits.write().unwrap().insert(expr, implicits_to_insert);
                        }
                    }

                    let mut arg_elab = self.check(argument, *_dom, depth, env, types)?;"""

c = c.replace(match_elab_end, repl_elab_end)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)


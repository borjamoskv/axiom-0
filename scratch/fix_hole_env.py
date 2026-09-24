import re

with open('src/elaborator.rs', 'r') as f:
    c = f.read()

# Replace the Hole elaboration to include the env in the spine!
match_hole = """            Expr::Hole => {
                if let Some(turbine) = self.turbine {
                    let meta_ty_id = turbine.new_meta();
                    let meta_val_id = turbine.new_meta();
                    turbine.expr_to_meta.write().unwrap().insert(expr, meta_val_id);
                    let meta_ty = Value::Meta(meta_ty_id, Vec::new());
                    Ok(Elaboration {
                        ty: meta_ty,
                        usages: vec![],
                    })
                } else {"""

repl_hole = """            Expr::Hole => {
                if let Some(turbine) = self.turbine {
                    let meta_ty_id = turbine.new_meta();
                    let meta_val_id = turbine.new_meta();
                    turbine.expr_to_meta.write().unwrap().insert(expr, meta_val_id);
                    let meta_ty = Value::Meta(meta_ty_id, env.to_vec());
                    Ok(Elaboration {
                        ty: meta_ty,
                        usages: vec![],
                    })
                } else {"""

c = c.replace(match_hole, repl_hole)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)

with open('src/eval.rs', 'r') as f:
    c = f.read()

match_eval_hole = """        Expr::Hole => {
            if let Some(turbine) = turbine {
                let map = turbine.expr_to_meta.read().unwrap();
                if let Some(&meta_id) = map.get(&expr) {
                    Value::Meta(meta_id, Vec::new())
                } else {"""
repl_eval_hole = """        Expr::Hole => {
            if let Some(turbine) = turbine {
                let map = turbine.expr_to_meta.read().unwrap();
                if let Some(&meta_id) = map.get(&expr) {
                    Value::Meta(meta_id, env.to_vec())
                } else {"""

c = c.replace(match_eval_hole, repl_eval_hole)

with open('src/eval.rs', 'w') as f:
    f.write(c)


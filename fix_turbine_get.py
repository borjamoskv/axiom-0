import re

with open('src/turbine.rs', 'r') as f:
    c = f.read()

match_solve = "    pub fn solve_meta(&self, meta: crate::ast::MetaId, value: Value) {"
repl_solve = """    pub fn get_meta(&self, meta: crate::ast::MetaId) -> Option<Value> {
        let ctx = self.meta_ctx.read().unwrap();
        if meta.0 < ctx.len() {
            ctx[meta.0].clone()
        } else {
            None
        }
    }

    pub fn solve_meta(&self, meta: crate::ast::MetaId, value: Value) {"""

c = c.replace(match_solve, repl_solve)

with open('src/turbine.rs', 'w') as f:
    f.write(c)

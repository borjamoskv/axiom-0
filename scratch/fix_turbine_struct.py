import re

with open('src/turbine.rs', 'r') as f:
    c = f.read()

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


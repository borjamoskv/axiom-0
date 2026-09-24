import re

with open('src/turbine.rs', 'r') as f:
    c = f.read()

match_struct = """pub struct TurbineEngine {
    arena: AtomicUsize,
    slots: Vec<SeqlockCell<usize, 4>>,
    meta_ctx: RwLock<Vec<Option<Value>>>,
    pub expr_to_meta: RwLock<std::collections::HashMap<crate::ast::ExprId, crate::ast::MetaId>>,
}"""

repl_struct = """pub struct TurbineEngine {
    arena: AtomicUsize,
    slots: Vec<SeqlockCell<usize, 4>>,
    meta_ctx: RwLock<Vec<Option<Value>>>,
    pub expr_to_meta: RwLock<std::collections::HashMap<crate::ast::ExprId, crate::ast::MetaId>>,
    pub inserted_implicits: RwLock<std::collections::HashMap<crate::ast::ExprId, Vec<crate::ast::MetaId>>>,
}"""

c = c.replace(match_struct, repl_struct)

match_init = """            meta_ctx: RwLock::new(Vec::new()),
            expr_to_meta: RwLock::new(std::collections::HashMap::new()),
        }"""

repl_init = """            meta_ctx: RwLock::new(Vec::new()),
            expr_to_meta: RwLock::new(std::collections::HashMap::new()),
            inserted_implicits: RwLock::new(std::collections::HashMap::new()),
        }"""

c = c.replace(match_init, repl_init)

with open('src/turbine.rs', 'w') as f:
    f.write(c)


sed -i '' 's/pub expr_to_meta: RwLock<std::collections::HashMap<crate::ast::ExprId, crate::ast::MetaId>>,/pub expr_to_meta: RwLock<std::collections::HashMap<crate::ast::ExprId, crate::ast::MetaId>>,\n    pub inserted_implicits: RwLock<std::collections::HashMap<crate::ast::ExprId, Vec<crate::ast::ExprId>>>,/g' src/turbine.rs

sed -i '' 's/expr_to_meta: RwLock::new(std::collections::HashMap::new()),/expr_to_meta: RwLock::new(std::collections::HashMap::new()),\n            inserted_implicits: RwLock::new(std::collections::HashMap::new()),/g' src/turbine.rs

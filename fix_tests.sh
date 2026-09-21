find examples tests -name "*.rs" | xargs sed -i '' 's/Expr::Pi { quantity/Expr::Pi { plicity: crate::ast::Plicity::Explicit, quantity/g'
find examples tests -name "*.rs" | xargs sed -i '' 's/Expr::Lambda { quantity/Expr::Lambda { plicity: crate::ast::Plicity::Explicit, quantity/g'
find examples tests -name "*.rs" | xargs sed -i '' 's/Expr::App { function/Expr::App { plicity: crate::ast::Plicity::Explicit, function/g'
find examples tests -name "*.rs" | xargs sed -i '' 's/Value::Pi(Quantity::/Value::Pi(crate::ast::Plicity::Explicit, Quantity::/g'
find examples tests -name "*.rs" | xargs sed -i '' 's/Value::Lam(Quantity::/Value::Lam(crate::ast::Plicity::Explicit, Quantity::/g'
find examples tests -name "*.rs" | xargs sed -i '' 's/Neutral::App(Box::new/Neutral::App(crate::ast::Plicity::Explicit, Box::new/g'
cargo test
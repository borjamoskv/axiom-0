sed -i '' 's/expr = self.ast.push(Expr::App {/expr = self.ast.push(Expr::App { plicity: crate::ast::Plicity::Explicit,/g' src/parser.rs
sed -i '' 's/Expr::Pi {/Expr::Pi { plicity: crate::ast::Plicity::Explicit,/g' src/parser.rs

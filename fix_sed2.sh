sed -i '' 's/Expr::App { plicity, function, argument } => {/Expr::App { plicity, function, argument } => {\n            println!("Evaluating App {:?}", expr);/g' src/eval.rs

import re

with open('src/eval.rs', 'r') as f:
    c = f.read()

match_eval_app = """        Expr::App { plicity, function, argument } => {
            let mut f = eval(ast, function, env, turbine);"""

repl_eval_app = """        Expr::App { plicity, function, argument } => {
            println!("Evaluating App {:?}", expr);
            let mut f = eval(ast, function, env, turbine);"""

c = c.replace(match_eval_app, repl_eval_app)

with open('src/eval.rs', 'w') as f:
    f.write(c)


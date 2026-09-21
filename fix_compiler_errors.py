import re

def process(filepath):
    with open(filepath, 'r') as f:
        c = f.read()
    
    # elaborator.rs
    c = c.replace('if let Expr::Lambda { quantity, body, plicity: _ } = term {', 'if let Expr::Lambda { plicity, quantity, body } = term {')
    c = c.replace('if let Expr::Lambda { quantity, body } = term {', 'if let Expr::Lambda { plicity, quantity, body } = term {')
    c = c.replace('if let Value::Pi(decl_q, dom, cod_closure) = expected {', 'if let Value::Pi(plic, decl_q, dom, cod_closure) = expected {')
    c = c.replace('let out_ty = Value::Pi(decl_q, dom, cod_closure);', 'let out_ty = Value::Pi(plic, decl_q, dom, cod_closure);')
    c = c.replace('Expr::Pi {\n                quantity: _,\n                domain,\n                codomain,\n            }', 'Expr::Pi {\n                plicity: _,\n                quantity: _,\n                domain,\n                codomain,\n            }')
    c = c.replace('Expr::App { function, argument } => {', 'Expr::App { plicity, function, argument } => {')
    c = c.replace('if let Value::Pi(decl_q, dom, cod) = f_elab.ty {', 'if let Value::Pi(plic, decl_q, dom, cod) = f_elab.ty {')
    
    # eval.rs
    c = c.replace('Expr::Pi { quantity, domain, codomain } => Value::Pi(\n            quantity,\n            Box::new(eval(ast, domain, env, turbine)),\n            Closure { env: env.to_vec(), body: codomain },\n        ),', 'Expr::Pi { plicity, quantity, domain, codomain } => Value::Pi(\n            plicity,\n            quantity,\n            Box::new(eval(ast, domain, env, turbine)),\n            Closure { env: env.to_vec(), body: codomain },\n        ),')
    c = c.replace('Value::Neutral(Neutral::App(Box::new(n), Box::new(Value::True)))', 'Value::Neutral(Neutral::App(crate::ast::Plicity::Explicit, Box::new(n), Box::new(Value::True)))')

    # parser.rs
    c = c.replace('expr = self.ast.push(Expr::App {\n                        function: expr,\n                        argument: arg,\n                    })?;', 'expr = self.ast.push(Expr::App {\n                        plicity: crate::ast::Plicity::Explicit,\n                        function: expr,\n                        argument: arg,\n                    })?;')
    c = c.replace('Expr::Pi {\n                            quantity: Quantity::Omega,\n                            domain,\n                            codomain,\n                        }', 'Expr::Pi {\n                            plicity: crate::ast::Plicity::Explicit,\n                            quantity: Quantity::Omega,\n                            domain,\n                            codomain,\n                        }')
    c = c.replace('.push_spanned_exact(Expr::Lambda { quantity, body }, span)', '.push_spanned_exact(Expr::Lambda { plicity: crate::ast::Plicity::Explicit, quantity, body }, span)')

    with open(filepath, 'w') as f:
        f.write(c)

for file in ['src/elaborator.rs', 'src/eval.rs', 'src/parser.rs']:
    process(file)


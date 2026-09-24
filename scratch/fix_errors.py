import re

def fix_file(filepath):
    with open(filepath, 'r') as f:
        c = f.read()

    # eval.rs
    c = c.replace('Value::Lam(_q, c)', 'Value::Lam(_p, _q, c)')
    c = c.replace('Neutral::App(Box::new(n.clone()), Box::new(var))', 'Neutral::App(crate::ast::Plicity::Explicit, Box::new(n.clone()), Box::new(var))')
    c = c.replace('Neutral::App(Box::new(n.clone()), Box::new(var.clone()))', 'Neutral::App(crate::ast::Plicity::Explicit, Box::new(n.clone()), Box::new(var.clone()))')
    c = c.replace('Expr::Lambda { quantity, body } => Value::Lam(\n            quantity,\n            Closure { env: env.to_vec(), body },', 'Expr::Lambda { plicity, quantity, body } => Value::Lam(\n            plicity,\n            quantity,\n            Closure { env: env.to_vec(), body },')

    # parser.rs
    c = c.replace('expr = self.ast.push(Expr::App {\n                        function: expr,\n                        argument: arg,\n                    })?;', 'expr = self.ast.push(Expr::App {\n                        plicity: crate::ast::Plicity::Explicit,\n                        function: expr,\n                        argument: arg,\n                    })?;')
    c = c.replace('expr = self.ast.push(Expr::Pi {\n                        quantity: Quantity::Omega,\n                        domain: expr,\n                        codomain,\n                    })?;', 'expr = self.ast.push(Expr::Pi {\n                        plicity: crate::ast::Plicity::Explicit,\n                        quantity: Quantity::Omega,\n                        domain: expr,\n                        codomain,\n                    })?;')
    c = c.replace('Expr::Pi {\n                            quantity: Quantity::Omega,\n                            domain,\n                            codomain,\n                        }', 'Expr::Pi {\n                            plicity: crate::ast::Plicity::Explicit,\n                            quantity: Quantity::Omega,\n                            domain,\n                            codomain,\n                        }')
    c = c.replace('self.ast\n                        .push_spanned_exact(Expr::Lambda { quantity, body }, span)?', 'self.ast\n                        .push_spanned_exact(Expr::Lambda { plicity: crate::ast::Plicity::Explicit, quantity, body }, span)?')

    # turbine.rs
    c = c.replace('Value::Pi(q, _, _) => Self {', 'Value::Pi(_, q, _, _) => Self {')
    c = c.replace('Value::Lam(_, closure) => closure.instantiate(ast, arg, Some(self)),', 'Value::Lam(_, _, closure) => closure.instantiate(ast, arg, Some(self)),')
    c = c.replace('Neutral::App(Box::new(neu), Box::new(arg))', 'Neutral::App(crate::ast::Plicity::Explicit, Box::new(neu), Box::new(arg))')

    with open(filepath, 'w') as f:
        f.write(c)

for file in ['src/eval.rs', 'src/parser.rs', 'src/turbine.rs']:
    fix_file(file)


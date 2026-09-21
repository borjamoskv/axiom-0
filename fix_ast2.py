with open('src/ast.rs', 'r') as f:
    c = f.read()

c = c.replace('pub enum Expr {', '#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub enum Plicity {\n    Explicit,\n    Implicit,\n}\n\n#[derive(Debug, Clone, PartialEq)]\npub enum Expr {')

# Fix Expr::App match in walk
c = c.replace('Expr::App { function, argument } | Expr::Pair { first: function, second: argument } => {', 'Expr::App { plicity: _, function, argument } | Expr::Pair { first: function, second: argument } => {')

with open('src/ast.rs', 'w') as f:
    f.write(c)

with open('src/parser.rs', 'r') as f:
    c = f.read()

c = c.replace('Expr::App {\n                        function: expr,\n                        argument: arg,\n                    }', 'Expr::App {\n                        plicity: crate::ast::Plicity::Explicit,\n                        function: expr,\n                        argument: arg,\n                    }')

c = c.replace('Expr::Pi {\n                            quantity: Quantity::Omega,\n                            domain,\n                            codomain,\n                        }', 'Expr::Pi {\n                            plicity: crate::ast::Plicity::Explicit,\n                            quantity: Quantity::Omega,\n                            domain,\n                            codomain,\n                        }')

with open('src/parser.rs', 'w') as f:
    f.write(c)

with open('src/elaborator.rs', 'r') as f:
    c = f.read()

c = c.replace('Value::Pi(q1, d1, c1), Value::Pi(q2, d2, c2)', 'Value::Pi(_, q1, d1, c1), Value::Pi(_, q2, d2, c2)')

with open('src/elaborator.rs', 'w') as f:
    f.write(c)


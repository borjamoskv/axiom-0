import re

with open('src/parser.rs', 'r') as f:
    c = f.read()

c = c.replace("""                    expr = self.ast.push(Expr::Pi {
                        quantity: Quantity::Omega,
                        domain: expr,
                        codomain,
                    })?;""", """                    expr = self.ast.push(Expr::Pi {
                        plicity: crate::ast::Plicity::Explicit,
                        quantity: Quantity::Omega,
                        domain: expr,
                        codomain,
                    })?;""")

with open('src/parser.rs', 'w') as f:
    f.write(c)


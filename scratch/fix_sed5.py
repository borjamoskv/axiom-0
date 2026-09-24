import re

with open('src/parser.rs', 'r') as f:
    c = f.read()

c = c.replace('expr = self.ast.push(Expr::Pi { plicity: crate::ast::Plicity::Explicit,\n                        plicity: crate::ast::Plicity::Explicit,\n                        quantity: Quantity::Omega,\n                        domain: expr,\n                        codomain,\n                    })?;', 'expr = self.ast.push(Expr::Pi { plicity: crate::ast::Plicity::Explicit,\n                        quantity: Quantity::Omega,\n                        domain: expr,\n                        codomain,\n                    })?;')

with open('src/parser.rs', 'w') as f:
    f.write(c)


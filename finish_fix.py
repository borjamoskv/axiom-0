import re

def process(filepath):
    with open(filepath, 'r') as f:
        c = f.read()

    # ast.rs fixes
    c = c.replace('#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub enum Plicity {', '#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub enum Plicity {')
    c = c.replace('self.node(id).map(|node| node.expression)', 'self.node(id).map(|node| node.expression.clone())')

    # parser.rs fixes
    c = c.replace('expr = self.ast.push(Expr::App {\n                        function: expr,\n                        argument: arg,\n                    })?;', 'expr = self.ast.push(Expr::App {\n                        plicity: crate::ast::Plicity::Explicit,\n                        function: expr,\n                        argument: arg,\n                    })?;')
    c = c.replace('Expr::Pi {\n                            quantity: Quantity::Omega,\n                            domain,\n                            codomain,\n                        }', 'Expr::Pi {\n                            plicity: crate::ast::Plicity::Explicit,\n                            quantity: Quantity::Omega,\n                            domain,\n                            codomain,\n                        }')

    with open(filepath, 'w') as f:
        f.write(c)

process('src/ast.rs')
process('src/parser.rs')


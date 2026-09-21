import re

with open('src/parser.rs', 'r') as f:
    c = f.read()

match_pi = """                    self.advance();
                    // Even a non-dependent function introduces a level. An empty
                    // name is inaccessible to source identifiers.
                    let codomain = self.parse_under_binder("")?;
                    expr = self.ast.push(Expr::Pi {
                        plicity: crate::ast::Plicity::Explicit,
                        quantity: Quantity::Omega,
                        domain: expr,
                        codomain,
                    })?;"""

# Let's check how we parse Pi. It's actually `parse_term`.

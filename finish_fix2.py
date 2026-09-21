import re

with open('src/parser.rs', 'r') as f:
    c = f.read()

c = c.replace('expr = self.ast.push(Expr::App {\n                                function: expr,\n                                argument: arg,\n                            })?;', 'expr = self.ast.push(Expr::App {\n                                plicity: crate::ast::Plicity::Explicit,\n                                function: expr,\n                                argument: arg,\n                            })?;')

c = c.replace('expr = self.ast.push(Expr::Pi {\n                                quantity: Quantity::Omega,\n                                domain,\n                                codomain,\n                            })?;', 'expr = self.ast.push(Expr::Pi {\n                                plicity: crate::ast::Plicity::Explicit,\n                                quantity: Quantity::Omega,\n                                domain,\n                                codomain,\n                            })?;')

c = c.replace('Expr::Pi {\n                                    quantity: Quantity::Omega,\n                                    domain,\n                                    codomain,\n                                }', 'Expr::Pi {\n                                    plicity: crate::ast::Plicity::Explicit,\n                                    quantity: Quantity::Omega,\n                                    domain,\n                                    codomain,\n                                }')

# Just to be sure, regex replace any missing plicity in parser.rs for App and Pi
c = re.sub(r'Expr::App\s*\{\s*function:\s*(.*?),\s*argument:\s*(.*?),\s*\}', r'Expr::App { plicity: crate::ast::Plicity::Explicit, function: \1, argument: \2 }', c)
c = re.sub(r'Expr::Pi\s*\{\s*quantity:\s*(.*?),\s*domain:\s*(.*?),\s*codomain:\s*(.*?),\s*\}', r'Expr::Pi { plicity: crate::ast::Plicity::Explicit, quantity: \1, domain: \2, codomain: \3 }', c)
c = re.sub(r'Expr::Pi\s*\{\s*quantity:\s*(.*?),\s*domain:\s*(.*?),\s*codomain(,?)\s*\}', r'Expr::Pi { plicity: crate::ast::Plicity::Explicit, quantity: \1, domain: \2, codomain\3 }', c)

with open('src/parser.rs', 'w') as f:
    f.write(c)


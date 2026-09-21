import re

with open('src/parser.rs', 'r') as f:
    c = f.read()

c = re.sub(r'Expr::Pi \{\n\s*quantity,\n\s*domain,\n\s*codomain,\n\s*\}', r'Expr::Pi { plicity: crate::ast::Plicity::Explicit, quantity, domain, codomain }', c)
c = re.sub(r'Expr::Lambda \{ quantity, body \}', r'Expr::Lambda { plicity: crate::ast::Plicity::Explicit, quantity, body }', c)
c = re.sub(r'Expr::App \{ function: expr, argument \}', r'Expr::App { plicity: crate::ast::Plicity::Explicit, function: expr, argument }', c)
c = re.sub(r'Expr::App \{ function: expr, argument: arg \}', r'Expr::App { plicity: crate::ast::Plicity::Explicit, function: expr, argument: arg }', c)

# find all Expr::App { function, argument }
c = re.sub(r'Expr::App \{\n\s*function(.*?),\n\s*argument(.*?),\n\s*\}', r'Expr::App {\n                                plicity: crate::ast::Plicity::Explicit,\n                                function\1,\n                                argument\2,\n                            }', c)

with open('src/parser.rs', 'w') as f:
    f.write(c)

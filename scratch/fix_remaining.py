import re

with open('tests/qtt_regressions.rs', 'r') as f:
    c = f.read()

c = c.replace('ast.push(Expr::Lambda { quantity, body }).unwrap()', 'ast.push(Expr::Lambda { plicity: micro_axiom_0::ast::Plicity::Explicit, quantity, body }).unwrap()')
c = c.replace('ast.push(Expr::App { function, argument }).unwrap()', 'ast.push(Expr::App { plicity: micro_axiom_0::ast::Plicity::Explicit, function, argument }).unwrap()')

with open('tests/qtt_regressions.rs', 'w') as f:
    f.write(c)

with open('examples/stress_10k_rigorous.rs', 'r') as f:
    c = f.read()

c = c.replace('let expected_ty = Value::Pi(\n                    Quantity::Omega,', 'let expected_ty = Value::Pi(\n                    micro_axiom_0::ast::Plicity::Explicit,\n                    Quantity::Omega,')

with open('examples/stress_10k_rigorous.rs', 'w') as f:
    f.write(c)

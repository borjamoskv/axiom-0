import os
import glob

def fix_file(filepath):
    with open(filepath, 'r') as f:
        c = f.read()

    # Match `Value::Pi(Quantity::Zero, ...)`
    c = c.replace('Value::Pi(Quantity::', 'Value::Pi(crate::ast::Plicity::Explicit, Quantity::')
    c = c.replace('Value::Pi(\n                        Quantity::', 'Value::Pi(\n                        crate::ast::Plicity::Explicit,\n                        Quantity::')
    
    # Match `Value::Lam(Quantity::Omega, ...)`
    c = c.replace('Value::Lam(Quantity::', 'Value::Lam(crate::ast::Plicity::Explicit, Quantity::')
    
    # Match `Neutral::App(Box::new(...)`
    c = c.replace('Neutral::App(Box::new', 'Neutral::App(crate::ast::Plicity::Explicit, Box::new')
    
    # Match Expr::Pi {
    c = c.replace('Expr::Pi {\n', 'Expr::Pi { plicity: crate::ast::Plicity::Explicit,\n')
    c = c.replace('Expr::Lambda {\n', 'Expr::Lambda { plicity: crate::ast::Plicity::Explicit,\n')
    c = c.replace('Expr::App {\n', 'Expr::App { plicity: crate::ast::Plicity::Explicit,\n')
    
    c = c.replace('Value::Pi(_, domain', 'Value::Pi(_, _, domain')

    with open(filepath, 'w') as f:
        f.write(c)

for root, _, files in os.walk('.'):
    for f in files:
        if f.endswith('.rs'):
            fix_file(os.path.join(root, f))


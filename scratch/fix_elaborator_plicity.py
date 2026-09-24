import re

with open('src/elaborator.rs', 'r') as f:
    c = f.read()

c = re.sub(r'Expr::Pi \{\n\s*quantity,\n\s*domain,\n\s*codomain,\n\s*\}', r'Expr::Pi { plicity, quantity, domain, codomain }', c)
c = re.sub(r'Expr::Lambda \{ quantity, body \}', r'Expr::Lambda { plicity, quantity, body }', c)
c = re.sub(r'Expr::App \{ function, argument \}', r'Expr::App { plicity, function, argument }', c)

# Value::Pi(q, _, codomain) -> Value::Pi(_, q, _, codomain)
c = re.sub(r'Value::Pi\(q, _, codomain\)', r'Value::Pi(_, q, _, codomain)', c)
c = re.sub(r'Value::Pi\(cod_q, _, cod\)', r'Value::Pi(_, cod_q, _, cod)', c)

# Value::Pi(_, domain_val, closure)
c = re.sub(r'Value::Pi\(_, domain_val, closure\)', r'Value::Pi(_, _, domain_val, closure)', c)
c = re.sub(r'Value::Lam\(_, closure\)', r'Value::Lam(_, _, closure)', c)

# Elaboration type
c = re.sub(r'Value::Pi\(\*quantity, Box::new\(domain_val\.clone\(\)\), closure\)', r'Value::Pi(*plicity, *quantity, Box::new(domain_val.clone()), closure)', c)

c = re.sub(r'Value::Pi\(quantity, Box::new\(ty\.clone\(\)\), codomain\)', r'Value::Pi(*plicity, quantity, Box::new(ty.clone()), codomain)', c)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)

import re

with open('src/eval.rs', 'r') as f:
    c = f.read()

# Value::Pi(Plicity, Quantity, Box<Value>, Closure)
c = re.sub(r'Pi\(Quantity, Box<Value>, Closure\)', r'Pi(crate::ast::Plicity, Quantity, Box<Value>, Closure)', c)

# Value::Lam(Plicity, Quantity, Closure)
c = re.sub(r'Lam\(Quantity, Closure\)', r'Lam(crate::ast::Plicity, Quantity, Closure)', c)

# Neutral::App(Plicity, Box<Neutral>, Box<Value>)
c = re.sub(r'App\(Box<Neutral>, Box<Value>\)', r'App(crate::ast::Plicity, Box<Neutral>, Box<Value>)', c)

with open('src/eval.rs', 'w') as f:
    f.write(c)

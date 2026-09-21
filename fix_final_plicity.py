import re

with open('src/ast.rs', 'r') as f:
    c = f.read()

# Fix double derive
c = c.replace('#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]\n\n#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\npub enum Plicity', '#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\npub enum Plicity')
c = c.replace('#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\npub enum Plicity {\n    Explicit,\n    Implicit,\n}\n\n#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\npub enum Plicity', '#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\npub enum Plicity')

c = re.sub(r'Expr::App \{ function, argument \} \| Expr::Pair \{ first: function, second: argument \}', r'Expr::App { plicity: _, function, argument } | Expr::Pair { first: function, second: argument }', c)

with open('src/ast.rs', 'w') as f:
    f.write(c)

with open('src/elaborator.rs', 'r') as f:
    c = f.read()

c = re.sub(r'Value::Pi\(q1, d1, c1\)', r'Value::Pi(_p1, q1, d1, c1)', c)
c = re.sub(r'Value::Pi\(q2, d2, c2\)', r'Value::Pi(_p2, q2, d2, c2)', c)
c = re.sub(r'Value::Pi\(decl_q, dom, cod_closure\)', r'Value::Pi(_plic, decl_q, dom, cod_closure)', c)
c = re.sub(r'let out_ty = Value::Pi\(_plic, decl_q, dom, cod_closure\);', r'let out_ty = Value::Pi(_plic, decl_q, dom, cod_closure);', c) # wait, cod_closure is the closure.

c = re.sub(r'Expr::Pi \{\n\s*quantity: _,\n\s*domain,\n\s*codomain,\n\s*\}', r'Expr::Pi { plicity: _, quantity: _, domain, codomain }', c)

c = re.sub(r'Value::Pi\(decl_q, dom, cod\)', r'Value::Pi(plic, decl_q, dom, cod)', c)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)

with open('src/eval.rs', 'r') as f:
    c = f.read()

c = re.sub(r'\*plicity', r'plicity', c)

with open('src/eval.rs', 'w') as f:
    f.write(c)

with open('src/parser.rs', 'r') as f:
    c = f.read()

c = re.sub(r'Expr::Pi \{\n\s*quantity,\n\s*domain,\n\s*codomain,\n\s*\}', r'Expr::Pi { plicity: crate::ast::Plicity::Explicit, quantity, domain, codomain }', c)

with open('src/parser.rs', 'w') as f:
    f.write(c)


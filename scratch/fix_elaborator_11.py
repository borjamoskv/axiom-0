with open('src/elaborator.rs', 'r') as f:
    c = f.read()

c = c.replace('if !unified {\n            return Err(Error::TypeMismatch {', 'if !unified {\n            println!("FAILED UNIFY: expr={:?} term={:?} ty={:?} exp={:?}", expr, term, elab.ty, expected);\n            return Err(Error::TypeMismatch {')

with open('src/elaborator.rs', 'w') as f:
    f.write(c)


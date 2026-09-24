with open('src/elaborator.rs', 'r') as f:
    c = f.read()

match_err = """            return Err(Error::TypeMismatch {
                expr,
                expected: format!("{:?}", expected),
                found: format!("{:?}", elab.ty),
            });"""

repl_err = """            println!("EXACT MISMATCH: expr={:?} elab={:?} exp={:?}", expr, elab.ty, expected);
            return Err(Error::TypeMismatch {
                expr,
                expected: format!("{:?}", expected),
                found: format!("{:?}", elab.ty),
            });"""

c = c.replace(match_err, repl_err)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)


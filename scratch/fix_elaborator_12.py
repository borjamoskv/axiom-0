with open('src/elaborator.rs', 'r') as f:
    c = f.read()

c = c.replace('if !unified_target {\n                    return Err(Error::TypeMismatch', 'if !unified_target {\n                    println!("FAILED TARGET UNIFY: {:?}", target_elab.ty);\n                    return Err(Error::TypeMismatch')
c = c.replace('if !unified_domain {\n                            return Err(Error::TypeMismatch', 'if !unified_domain {\n                            println!("FAILED DOMAIN UNIFY: {:?}", **domain);\n                            return Err(Error::TypeMismatch')

with open('src/elaborator.rs', 'w') as f:
    f.write(c)


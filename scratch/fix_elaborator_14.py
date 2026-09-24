with open('src/elaborator.rs', 'r') as f:
    c = f.read()

c = c.replace('if !unified_target {\n                    println!("FAILED TARGET UNIFY: {:?}", target_elab.ty);\n                    return Err(Error::TypeMismatch {', 'if !unified_target {\n                    panic!("TARGET UNIFY FAILED");')
c = c.replace('if !unified_domain {\n                            println!("FAILED DOMAIN UNIFY: {:?}", **domain);\n                            return Err(Error::TypeMismatch {', 'if !unified_domain {\n                            panic!("DOMAIN UNIFY FAILED");')
c = c.replace('if !unified {\n            return Err(Error::TypeMismatch {', 'if !unified {\n            panic!("CHECK UNIFY FAILED for expr {:?} expected {:?}", expr, expected);')
c = c.replace('if !unified {\n                        return Err(Error::TypeMismatch {', 'if !unified {\n                        panic!("S UNIFY FAILED");')

with open('src/elaborator.rs', 'w') as f:
    f.write(c)


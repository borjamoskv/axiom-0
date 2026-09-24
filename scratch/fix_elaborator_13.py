with open('src/elaborator.rs', 'r') as f:
    c = f.read()

import re

c = re.sub(r'return Err\(Error::TypeMismatch \{', r'panic!("TypeMismatch at line {}: expr={:?} expected={:?} found={:?}", line!(), expr, expected, if True { "unknown" } else { "unknown" });\nreturn Err(Error::TypeMismatch {', c)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)


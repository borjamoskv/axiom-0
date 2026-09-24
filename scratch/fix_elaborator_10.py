with open('src/elaborator.rs', 'r') as f:
    c = f.read()

c = c.replace('*s_body1', 's_body1')
c = c.replace('*s_body2', 's_body2')
c = c.replace('return Err(Error::ExpectedFunction { expr: s_body1 });', 'return Err(Error::ExpectedFunction { expr: s_body1, found: "Expected an inner function for ih".into() });')

with open('src/elaborator.rs', 'w') as f:
    f.write(c)


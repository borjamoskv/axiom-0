with open('src/elaborator.rs', 'r') as f:
    c = f.read()

c = c.replace('expected: Value::NatType,', 'expected: format!("{:?}", Value::NatType),')
c = c.replace('found: target_elab.ty,', 'found: format!("{:?}", target_elab.ty),')
c = c.replace('found: (**domain).clone(),', 'found: format!("{:?}", (**domain).clone()),')
c = c.replace('Some(self.turbine)', 'self.turbine')

with open('src/elaborator.rs', 'w') as f:
    f.write(c)

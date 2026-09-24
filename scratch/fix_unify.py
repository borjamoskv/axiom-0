with open('src/elaborator.rs', 'r') as f:
    c = f.read()

match_unify = """            (Value::Unit, Value::Unit) => true,
            (Value::UnitType, Value::UnitType) => true,"""

repl_unify = """            (Value::Unit, Value::Unit) => true,
            (Value::UnitType, Value::UnitType) => true,
            (Value::NatType, Value::NatType) => true,
            (Value::Zero, Value::Zero) => true,
            (Value::Succ(n1), Value::Succ(n2)) => Self::unify(ast, &n1, &n2, depth, turbine),"""

c = c.replace(match_unify, repl_unify)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)


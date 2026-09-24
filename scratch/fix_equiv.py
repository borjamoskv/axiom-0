with open('src/eval.rs', 'r') as f:
    c = f.read()

match_equiv = """        (Value::Unit, Value::Unit) => true,
        (Value::UnitType, Value::UnitType) => true,"""
repl_equiv = """        (Value::Unit, Value::Unit) => true,
        (Value::UnitType, Value::UnitType) => true,
        (Value::NatType, Value::NatType) => true,
        (Value::Zero, Value::Zero) => true,
        (Value::Succ(n1), Value::Succ(n2)) => equiv(ast, n1, n2, depth),"""

c = c.replace(match_equiv, repl_equiv)

with open('src/eval.rs', 'w') as f:
    f.write(c)


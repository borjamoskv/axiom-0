with open('src/eval.rs', 'r') as f:
    lines = f.readlines()

new_lines = []
in_app = False
for line in lines:
    if "Expr::Meta(id) => Value::Meta(id, Vec::new())," in line:
        new_lines.append(line)
        new_lines.append("        Expr::NatType => Value::NatType,\n")
        new_lines.append("        Expr::Zero => Value::Zero,\n")
        new_lines.append("        Expr::Succ(n) => Value::Succ(Box::new(eval(ast, n, env, turbine))),\n")
        new_lines.append("        Expr::Ind { mot, z, s, target } => {\n")
        new_lines.append("            let mot_val = eval(ast, mot, env, turbine);\n")
        new_lines.append("            let z_val = eval(ast, z, env, turbine);\n")
        new_lines.append("            let s_val = eval(ast, s, env, turbine);\n")
        new_lines.append("            let target_val = eval(ast, target, env, turbine);\n")
        new_lines.append("            eval_ind(ast, mot_val, z_val, s_val, target_val, turbine)\n")
        new_lines.append("        }\n")
    else:
        new_lines.append(line)

with open('src/eval.rs', 'w') as f:
    f.writelines(new_lines)

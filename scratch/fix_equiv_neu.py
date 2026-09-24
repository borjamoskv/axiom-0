with open('src/eval.rs', 'r') as f:
    c = f.read()

target = """        (Neutral::Fst(n1), Neutral::Fst(n2)) => equiv_neu(ast, n1, n2, depth),
        (Neutral::Snd(n1), Neutral::Snd(n2)) => equiv_neu(ast, n1, n2, depth),
        _ => false,"""

replacement = """        (Neutral::Fst(n1), Neutral::Fst(n2)) => equiv_neu(ast, n1, n2, depth),
        (Neutral::Snd(n1), Neutral::Snd(n2)) => equiv_neu(ast, n1, n2, depth),
        (Neutral::Ind(m1, z1, s1, t1), Neutral::Ind(m2, z2, s2, t2)) => {
            equiv(ast, m1, m2, depth)
                && equiv(ast, z1, z2, depth)
                && equiv(ast, s1, s2, depth)
                && equiv_neu(ast, t1, t2, depth)
        }
        _ => false,"""

c = c.replace(target, replacement)
with open('src/eval.rs', 'w') as f:
    f.write(c)

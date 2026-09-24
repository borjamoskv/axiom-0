with open('src/elaborator.rs', 'r') as f:
    c = f.read()

c = c.replace('*mot_body1', 'mot_body1')

with open('src/elaborator.rs', 'w') as f:
    f.write(c)


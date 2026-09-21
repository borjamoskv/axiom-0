import re

with open('tests/implicits.rs', 'r') as f:
    c = f.read()

c = c.replace('"let identity : {A : type} -> (a : A) -> A = fn {A} -> fn a -> a"', '"let identity : fn {A : type} -> fn (a : A) -> A = fn {A} -> fn a -> a"')

with open('tests/implicits.rs', 'w') as f:
    f.write(c)


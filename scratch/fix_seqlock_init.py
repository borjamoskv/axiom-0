with open('tests/stress.rs', 'r') as f:
    c = f.read()

c = c.replace('let cell = Arc::new(SeqlockCell::new([0usize; 8]));', 'let cell = Arc::new(SeqlockCell::new([0, 1, 2, 3, 4, 5, 6, 7]));')

with open('tests/stress.rs', 'w') as f:
    f.write(c)


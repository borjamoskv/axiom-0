with open('tests/stress.rs', 'r') as f:
    c = f.read()

old_test = """    let src = "ind(fn n -> Nat, Z, fn n -> fn ih -> ind(fn m -> Nat, ih, fn k -> fn acc -> S(acc), S(S(S(S(S(Z)))))), S(S(S(S(Z)))))";
    
    let mut ast = Ast::new();
    let expr = {
        let mut lexer = micro_axiom_0::lexer::Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = micro_axiom_0::parser::Parser::new(&tokens, &mut ast, &[]);
        parser.parse_expression().unwrap()
    };

    // 1. Synthesize & Type Check
    let elab = elaborator::synthesize(&ast, expr, &[]).expect("Type check failed for Peano multiplication");
    assert_eq!(elab.ty, Value::NatType);

    // 2. Evaluate via NbE
    let val = eval(&ast, expr, &[], None);

    // Count successors
    let mut current = &val;
    let mut count = 0;
    while let Value::Succ(inner) = current {
        count += 1;
        current = inner;
    }
    assert_eq!(*current, Value::Zero);
    assert_eq!(count, 20, "Peano multiplication 4 * 5 must evaluate to 20");

    let elapsed = start.elapsed();
    println!("-> Elaborated and normalized Peano 4 * 5 = 20 in {:?}", elapsed);"""

new_test = """    fn peano(n: usize) -> String {
        let mut s = "Z".to_string();
        for _ in 0..n {
            s = format!("S({})", s);
        }
        s
    }
    let a = 10;
    let b = 10;
    let src = format!(
        "ind(fn n -> Nat, Z, fn n -> fn ih -> ind(fn m -> Nat, ih, fn k -> fn acc -> S(acc), {}), {})",
        peano(b),
        peano(a)
    );
    
    let mut ast = Ast::new();
    let expr = {
        let mut lexer = micro_axiom_0::lexer::Lexer::new(&src);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = micro_axiom_0::parser::Parser::new(&tokens, &mut ast, &[]);
        parser.parse_expression().unwrap()
    };

    // 1. Synthesize & Type Check
    let elab = elaborator::synthesize(&ast, expr, &[]).expect("Type check failed for Peano multiplication");
    assert_eq!(elab.ty, Value::NatType);

    // 2. Evaluate via NbE
    let val = eval(&ast, expr, &[], None);

    // Count successors
    let mut current = &val;
    let mut count = 0;
    while let Value::Succ(inner) = current {
        count += 1;
        current = inner;
    }
    assert_eq!(*current, Value::Zero);
    assert_eq!(count, a * b, "Peano multiplication 10 * 10 must evaluate to 100");

    let elapsed = start.elapsed();
    println!("-> Elaborated and normalized Peano {} * {} = {} in {:?}", a, b, a * b, elapsed);"""

c = c.replace(old_test, new_test)
with open('tests/stress.rs', 'w') as f:
    f.write(c)


with open('tests/stress.rs', 'r') as f:
    c = f.read()

old_test9_part = """    // Generate streaming multi-command source: 1,000 let commands
    let count = 1000;
    let mut src = String::with_capacity(count * 32);
    for i in 0..count {
        src.push_str(&format!("let x_{} = ();\\n", i));
    }

    let bytes = src.len();

    // 1. Lexer throughput
    let lex_start = Instant::now();
    let mut lexer = micro_axiom_0::lexer::Lexer::new(&src);
    let tokens = lexer.tokenize_all().expect("Lexing failed");
    let lex_time = lex_start.elapsed();

    // 2. Parser streaming throughput
    let parse_start = Instant::now();
    let mut ast = Ast::new();
    let mut names = Vec::new();
    let mut cursor = 0;
    while cursor < tokens.len() {
        let mut parser = micro_axiom_0::parser::Parser::new(&tokens[cursor..], &mut ast, &names);
        match parser.parse_command() {
            Ok(micro_axiom_0::ast::Command::Let { name, .. }) => {
                cursor += parser.cursor_position();
                names.push(name);
            }
            Ok(_) => {
                cursor += parser.cursor_position();
            }
            Err(_) => break,
        }
    }
    let _parse_time = parse_start.elapsed();"""

new_test9_part = """    // Generate flat application spine: () () () ... () (3,000 arguments in a flat loop)
    let count = 3000;
    let mut src = String::with_capacity(count * 4);
    src.push_str("()");
    for _ in 0..count {
        src.push_str(" ()");
    }

    let bytes = src.len();

    // 1. Lexer throughput
    let lex_start = Instant::now();
    let mut lexer = micro_axiom_0::lexer::Lexer::new(&src);
    let tokens = lexer.tokenize_all().expect("Lexing failed");
    let lex_time = lex_start.elapsed();

    // 2. Parser flat spine throughput (iterative while loop in parse_expression_inner)
    let parse_start = Instant::now();
    let mut ast = Ast::new();
    let mut parser = micro_axiom_0::parser::Parser::new(&tokens, &mut ast, &[]);
    let _ = parser.parse_expression().expect("Parsing failed");
    let _parse_time = parse_start.elapsed();"""

c = c.replace(old_test9_part, new_test9_part)
with open('tests/stress.rs', 'w') as f:
    f.write(c)


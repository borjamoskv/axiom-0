with open('tests/stress.rs', 'r') as f:
    c = f.read()

# Replace Test 9
old_test9 = """/// 9. Lexer & Parser Throughput Stress: Tokenizing and parsing 50,000 bytes of nested AST expressions.
#[test]
fn test_stress_lexer_parser_massive_throughput() {
    println!("\\n=== [STRESS 9/12] Lexer & Parser Streaming Throughput (50 KB buffer) ===");
    let start = Instant::now();

    // Generate large synthetic program with 500 let-bindings
    let mut src = String::with_capacity(64 * 1024);
    for i in 0..500 {
        src.push_str(&format!(
            "let x_{} : fn (a: UnitType) -> UnitType = fn a -> a;\\n",
            i
        ));
    }
    src.push_str("x_499\\n");

    let bytes = src.len();

    // 1. Lexer throughput
    let lex_start = Instant::now();
    let mut lexer = micro_axiom_0::lexer::Lexer::new(&src);
    let tokens = lexer.tokenize_all().expect("Lexing failed");
    let lex_time = lex_start.elapsed();

    // 2. Parser throughput
    let parse_start = Instant::now();
    let mut ast = Ast::new();
    let mut parser = micro_axiom_0::parser::Parser::new(&tokens, &mut ast, &[]);
    let _ = parser.parse_expression().expect("Parsing failed");
    let parse_time = parse_start.elapsed();

    let total_time = start.elapsed();
    let mb_lex = (bytes as f64) / (lex_time.as_secs_f64() * 1024.0 * 1024.0);
    let mb_total = (bytes as f64) / (total_time.as_secs_f64() * 1024.0 * 1024.0);

    println!(
        "-> Processed {} bytes: Lexer {:.2} MB/s, Total Pipeline {:.2} MB/s (AST nodes: {}) in {:?}",
        bytes,
        mb_lex,
        mb_total,
        ast.expression_count(),
        total_time
    );
}"""

new_test9 = """/// 9. Lexer & Parser Throughput Stress: Tokenizing and parsing 50,000 bytes of nested AST expressions.
#[test]
fn test_stress_lexer_parser_massive_throughput() {
    println!("\\n=== [STRESS 9/12] Lexer & Parser Streaming Throughput (50 KB buffer) ===");
    let start = Instant::now();

    // Generate a deeply nested tuple expression: ((), ((), ((), ... ((), ()))) ...)
    let depth = 3000;
    let mut src = String::with_capacity(depth * 6);
    for _ in 0..depth {
        src.push_str("((), ");
    }
    src.push_str("()");
    for _ in 0..depth {
        src.push(')');
    }

    let bytes = src.len();

    // 1. Lexer throughput
    let lex_start = Instant::now();
    let mut lexer = micro_axiom_0::lexer::Lexer::new(&src);
    let tokens = lexer.tokenize_all().expect("Lexing failed");
    let lex_time = lex_start.elapsed();

    // 2. Parser throughput
    let parse_start = Instant::now();
    let mut ast = Ast::new();
    let mut parser = micro_axiom_0::parser::Parser::new(&tokens, &mut ast, &[]);
    let _ = parser.parse_expression().expect("Parsing failed");
    let _parse_time = parse_start.elapsed();

    let total_time = start.elapsed();
    let mb_lex = (bytes as f64) / (lex_time.as_secs_f64() * 1024.0 * 1024.0);
    let mb_total = (bytes as f64) / (total_time.as_secs_f64() * 1024.0 * 1024.0);

    println!(
        "-> Processed {} bytes: Lexer {:.2} MB/s, Total Pipeline {:.2} MB/s (AST nodes: {}) in {:?}",
        bytes,
        mb_lex,
        mb_total,
        ast.expression_count(),
        total_time
    );
}"""

# Replace Test 12
old_test12 = """/// 12. Metavariable Hole Unification Stress: Resolving interconnected holes via first-order unification.
#[test]
fn test_stress_metavariable_hole_cascade_unification() {
    println!("\\n=== [STRESS 12/12] Metavariable Hole Unification & Substitution Cascade ===");
    let start = Instant::now();

    // Hole resolution via elaborator: (\\x -> x) : fn (a: ?) -> UnitType
    let mut ast = Ast::new();
    let hole = ast.push(Expr::Hole).unwrap();
    let unit_ty = ast.push(Expr::UnitType).unwrap();
    let pi_with_hole = ast
        .push(Expr::Pi {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            domain: hole,
            codomain: unit_ty,
        })
        .unwrap();

    let turbine = TurbineEngine::for_ast(&ast);
    let expected_val = eval(&ast, pi_with_hole, &[], Some(&turbine));

    let var0 = ast.push(Expr::Var(Level(0))).unwrap();
    let lam = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: var0,
        })
        .unwrap();

    let elab = elaborator::check_with_turbine(&ast, lam, expected_val, &[], &turbine)
        .expect("Hole unification failed");
    assert!(elab.usages.is_empty());

    let elapsed = start.elapsed();
    println!("-> Solved metavariable hole and unified polymorphic closure in {:?}", elapsed);
}"""

new_test12 = """/// 12. Metavariable Hole Unification Stress: Resolving interconnected holes via first-order unification.
#[test]
fn test_stress_metavariable_hole_cascade_unification() {
    println!("\\n=== [STRESS 12/12] Metavariable Hole Unification & Substitution Cascade ===");
    let start = Instant::now();

    // Resolving hole through polymorphic instantiation: identity ? True
    let script1 = "let id : fn (A : type) -> fn (a : A) -> A = fn A -> fn a -> a";
    let script2 = "let res = id ? True";

    let mut ast = Ast::new();
    let mut names = Vec::new();

    let cmd1 = {
        let mut lexer = micro_axiom_0::lexer::Lexer::new(script1);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = micro_axiom_0::parser::Parser::new(&tokens, &mut ast, &names);
        parser.parse_command().unwrap()
    };
    names.push("id".to_string());

    let cmd2 = {
        let mut lexer = micro_axiom_0::lexer::Lexer::new(script2);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = micro_axiom_0::parser::Parser::new(&tokens, &mut ast, &names);
        parser.parse_command().unwrap()
    };

    let turbine = TurbineEngine::for_ast(&ast);
    let mut env = Vec::new();
    let mut types = Vec::new();

    if let micro_axiom_0::ast::Command::Let { ty, term, .. } = cmd1 {
        let ty_val = eval(&ast, ty.unwrap(), &env, Some(&turbine));
        elaborator::check_with_turbine(&ast, term, ty_val.clone(), &types, &turbine).unwrap();
        let term_val = eval(&ast, term, &env, Some(&turbine));
        env.push(term_val);
        types.push(ty_val);
    }

    if let micro_axiom_0::ast::Command::Let { term, .. } = cmd2 {
        let elab = elaborator::synthesize_with_turbine(&ast, term, &types, &turbine)
            .expect("Hole synthesis with Turbine failed");
        let final_type = turbine.force(&ast, elab.ty);
        assert_eq!(final_type, Value::Bool);

        let term_val = eval(&ast, term, &env, Some(&turbine));
        let final_val = turbine.force(&ast, term_val);
        assert_eq!(final_val, Value::True);
    }

    let elapsed = start.elapsed();
    println!("-> Solved metavariable hole and unified polymorphic closure in {:?}", elapsed);
}"""

c = c.replace(old_test9, new_test9)
c = c.replace(old_test12, new_test12)

with open('tests/stress.rs', 'w') as f:
    f.write(c)


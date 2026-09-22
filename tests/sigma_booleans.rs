use micro_axiom_0::ast::Ast;
use micro_axiom_0::eval::Value;
use micro_axiom_0::parser::Parser;
use micro_axiom_0::lexer::Lexer;
use micro_axiom_0::elaborator;

#[test]
fn test_booleans() {
    let src = "if True then False else True";
    let mut ast = Ast::new();
    let expr = {
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(&tokens, &mut ast, &[]);
        parser.parse_expression().unwrap()
    };

    let elab = elaborator::check(&ast, expr, Value::Bool, &[]).unwrap();
    assert_eq!(elab.ty, Value::Bool);

    let val = micro_axiom_0::eval::eval(&ast, expr, &[], None);
    assert_eq!(val, Value::False);
}

#[test]
fn test_sigma_pairs() {
    let _src = "let t : sigma (x : Bool) * Bool = (True, False); t.1";
    let mut ast = Ast::new();
    
    let (ty_expr, term_expr) = {
        let mut lexer = Lexer::new("sigma (x : Bool) * Bool");
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(&tokens, &mut ast, &[]);
        let ty_expr = parser.parse_expression().unwrap();
        
        let mut lexer2 = Lexer::new("(True, False)");
        let tokens2 = lexer2.tokenize_all().unwrap();
        let mut parser2 = Parser::new(&tokens2, &mut ast, &[]);
        let term_expr = parser2.parse_expression().unwrap();
        (ty_expr, term_expr)
    };

    let ty_val = micro_axiom_0::eval::eval(&ast, ty_expr, &[], None);
    elaborator::check(&ast, term_expr, ty_val.clone(), &[]).unwrap();
    
    let term_val = micro_axiom_0::eval::eval(&ast, term_expr, &[], None);
    assert_eq!(term_val, Value::Pair(Box::new(Value::True), Box::new(Value::False)));
    
    let proj_expr = {
        let mut lexer3 = Lexer::new("t.1");
        let tokens3 = lexer3.tokenize_all().unwrap();
        let mut parser3 = Parser::new(&tokens3, &mut ast, &["t".to_string()]);
        parser3.parse_expression().unwrap()
    };
    
    let env = vec![term_val];
    let types = vec![ty_val];
    let proj_elab = elaborator::synthesize_in_env(&ast, proj_expr, &env, &types).unwrap();
    assert_eq!(proj_elab.ty, Value::Bool);
    
    let proj_val = micro_axiom_0::eval::eval(&ast, proj_expr, &env, None);
    assert_eq!(proj_val, Value::True);
}

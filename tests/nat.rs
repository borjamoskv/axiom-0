use micro_axiom_0::ast::Ast;
use micro_axiom_0::eval::Value;
use micro_axiom_0::parser::Parser;
use micro_axiom_0::lexer::Lexer;
use micro_axiom_0::elaborator;

#[test]
fn test_nat_induction() {
    let src = "ind(fn n -> Nat, S(Z), fn n -> fn ih -> S(ih), S(S(Z)))";
    let mut ast = Ast::new();
    let expr = {
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(&tokens, &mut ast, &[]);
        parser.parse_expression().unwrap()
    };
    let elab = elaborator::synthesize(&ast, expr, &[]).unwrap();
    assert_eq!(elab.ty, Value::NatType);

    let val = micro_axiom_0::eval::eval(&ast, expr, &[], None);
    assert_eq!(val, Value::Succ(Box::new(Value::Succ(Box::new(Value::Succ(Box::new(Value::Zero)))))));
}

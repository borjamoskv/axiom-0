use micro_axiom_0::ast::*;
use micro_axiom_0::eval::Value;
use micro_axiom_0::lexer::Lexer;
use micro_axiom_0::parser::Parser;
use micro_axiom_0::elaborator;

#[test]
fn test_implicit_arguments() {
    let script1 = "let identity : fn {A : type} -> fn (a : A) -> A = fn {A} -> fn a -> a";
    let script2 = "let apply = identity True";
    
    let mut ast = Ast::new();
    let mut names = Vec::new();

    let term_cmd1 = {
        let mut lexer = Lexer::new(script1);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(&tokens, &mut ast, &names);
        parser.parse_command().unwrap()
    };
    
    names.push("identity".to_string());
    
    let term_cmd2 = {
        let mut lexer = Lexer::new(script2);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(&tokens, &mut ast, &names);
        parser.parse_command().unwrap()
    };
    
    let turbine = micro_axiom_0::turbine::TurbineEngine::for_ast(&ast);
    let mut env = Vec::new();
    let mut types = Vec::new();

    if let Command::Let { name: _name, ty, term } = term_cmd1 {
        let ty_val = micro_axiom_0::eval::eval(&ast, ty.unwrap(), &env, Some(&turbine));
        elaborator::check_with_turbine(&ast, term, ty_val.clone(), &types, &turbine).unwrap();
        let term_val = micro_axiom_0::eval::eval(&ast, term, &env, Some(&turbine));
        env.push(term_val);
        types.push(ty_val);
    }

    if let Command::Let { name: _name, ty: _, term } = term_cmd2 {
        let elab = elaborator::synthesize_with_turbine(&ast, term, &types, &turbine).unwrap();
        
        let final_type = turbine.force(&ast, elab.ty);
        assert_eq!(final_type, Value::Bool);
        
        let term_val = micro_axiom_0::eval::eval(&ast, term, &env, Some(&turbine));
        let final_val = turbine.force(&ast, term_val);
        assert_eq!(final_val, Value::True);
    }
}

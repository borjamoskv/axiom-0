use std::io::{self, Write};
use crate::ast::Ast;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::elaborator::{synthesize, check};
use crate::eval::eval;

pub fn start_repl() {
    println!("=== AXIOM-0 REPL (Singularidad NbE) ===");
    println!("Modo de Alta Exergía Activo. Presiona Ctrl+C para salir.");
    
    let mut ast = Ast::new();
    let global_env = vec![];
    
    loop {
        print!("> ");
        io::stdout().flush().expect("Fallo de entropía en I/O");
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() || input.trim().is_empty() {
            continue;
        }
        
        let input = input.trim();
        if input == "exit" || input == "quit" {
            break;
        }

        // 1. Lexing
        let mut lexer = Lexer::new(input);
        let tokens = match lexer.tokenize_all() {
            Ok(t) => t,
            Err(e) => {
                println!("Error Léxico: {:?}", e);
                continue;
            }
        };
        if tokens.is_empty() { continue; }

        // 2. Parsing
        let mut parser = Parser::new(&tokens, &mut ast);
        match parser.parse_expression() {
            Ok(expr_id) => {
                // 3. Elaboración (Inferencia de tipos)
                match synthesize(&ast, expr_id) {
                    Ok(elaboration) => {
                        // 4. Evaluación Semántica (Forma Normal)
                        let value = eval(&ast, expr_id, &global_env);
                        println!("==> Val: {:?}", value);
                        println!("    Typ: {:?}", elaboration.ty);
                    }
                    Err(e) => println!("Error de Elaboración: {:?}", e),
                }
            }
            Err(e) => println!("Error de Sintaxis: {}", e),
        }
    }
}

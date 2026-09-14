use crate::ast::{Ast, Command};
use crate::elaborator::{check, synthesize};
use crate::eval::eval;
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::io::{self, Write};

pub fn start_repl() {
    println!("=== AXIOM-0 REPL (Singularidad NbE) ===");
    println!("Modo de Alta Exergía Activo. Presiona Ctrl+C para salir.");

    let mut ast = Ast::new();
    let mut global_env = vec![];
    let mut global_types = vec![];
    let mut global_names = vec![];

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

        let mut lexer = Lexer::new(input);
        let tokens = match lexer.tokenize_all() {
            Ok(t) => t,
            Err(e) => {
                println!("Error Léxico: {:?}", e);
                continue;
            }
        };
        if tokens.is_empty() {
            continue;
        }

        let mut parser = Parser::new(&tokens, &mut ast, &global_names);
        match parser.parse_command() {
            Ok(Command::Eval(expr_id)) => match synthesize(&ast, expr_id, &global_types) {
                Ok(elaboration) => {
                    let value = eval(&ast, expr_id, &global_env);
                    println!("==> Val: {:?}", value);
                    println!("    Typ: {:?}", elaboration.ty);
                }
                Err(e) => println!("Error de Elaboración: {:?}", e),
            },
            Ok(Command::Let { name, ty, term }) => {
                let term_ty = if let Some(t) = ty {
                    let expected_ty_val = eval(&ast, t, &global_env);
                    match check(&ast, term, expected_ty_val.clone(), &global_types) {
                        Ok(_) => expected_ty_val,
                        Err(e) => {
                            println!("Error de Tipado en Let: {:?}", e);
                            continue;
                        }
                    }
                } else {
                    match synthesize(&ast, term, &global_types) {
                        Ok(elaboration) => elaboration.ty,
                        Err(e) => {
                            println!("Error de Inferencia en Let: {:?}", e);
                            continue;
                        }
                    }
                };

                let value = eval(&ast, term, &global_env);

                global_names.push(name.clone());
                global_env.push(value.clone());
                global_types.push(term_ty.clone());

                println!("{} definido.", name);
                println!("==> Val: {:?}", value);
                println!("    Typ: {:?}", term_ty);
            }
            Err(e) => println!("Error de Sintaxis: {}", e),
        }
    }
}

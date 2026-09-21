use crate::ast::{Ast, Command};
use crate::elaborator::{check_in_env, synthesize_in_env};
use crate::eval::{Value, eval};
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::io::{self, BufRead, Write};

/// Runs the interactive interpreter using the supplied input and output streams.
///
/// End of input exits normally. Stream errors are returned to the caller, and
/// invalid commands leave the names, values, and types of globals unchanged.
pub fn run_repl<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> io::Result<()> {
    writeln!(writer, "=== AXIOM-0 REPL ===")?;
    writeln!(
        writer,
        "Escribe expresiones o definiciones let. Usa exit, quit o Ctrl+D para salir."
    )?;

    let mut ast = Ast::new();
    let mut global_env = vec![];
    let mut global_types = vec![];
    let mut global_names = vec![];

    loop {
        write!(writer, "> ")?;
        writer.flush()?;

        let mut input = String::new();
        if reader.read_line(&mut input)? == 0 {
            return Ok(());
        }
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if input == "exit" || input == "quit" {
            return Ok(());
        }

        let mut lexer = Lexer::new(input);
        let tokens = match lexer.tokenize_all() {
            Ok(tokens) => tokens,
            Err(error) => {
                writeln!(writer, "Error léxico: {error:?}")?;
                continue;
            }
        };
        if tokens.is_empty() {
            continue;
        }

        let mut parser = Parser::new(&tokens, &mut ast, &global_names);
        match parser.parse_command() {
            Ok(Command::Eval(expr_id)) => {
                match synthesize_in_env(&ast, expr_id, &global_env, &global_types) {
                    Ok(elaboration) => {
                        let value = eval(&ast, expr_id, &global_env, None);
                        writeln!(writer, "==> Val: {value:?}")?;
                        writeln!(writer, "    Typ: {:?}", elaboration.ty)?;
                    }
                    Err(error) => writeln!(writer, "Error de elaboración: {error}")?,
                }
            }
            Ok(Command::Let { name, ty, term }) => {
                let term_ty = if let Some(annotation) = ty {
                    // An annotation must itself have a universe type before it
                    // can be evaluated safely and used as an expected type.
                    match synthesize_in_env(&ast, annotation, &global_env, &global_types) {
                        Ok(elaboration) if matches!(elaboration.ty, Value::Universe(_)) => {}
                        Ok(_) => {
                            writeln!(
                                writer,
                                "Error de tipo en let: la anotación debe ser un tipo."
                            )?;
                            continue;
                        }
                        Err(error) => {
                            writeln!(writer, "Error de tipo en let: {error}")?;
                            continue;
                        }
                    }
                    let expected = eval(&ast, annotation, &global_env, None);
                    match check_in_env(&ast, term, expected.clone(), &global_env, &global_types) {
                        Ok(_) => expected,
                        Err(error) => {
                            writeln!(writer, "Error de tipo en let: {error}")?;
                            continue;
                        }
                    }
                } else {
                    match synthesize_in_env(&ast, term, &global_env, &global_types) {
                        Ok(elaboration) => elaboration.ty,
                        Err(error) => {
                            writeln!(writer, "Error de inferencia en let: {error}")?;
                            continue;
                        }
                    }
                };

                let value = eval(&ast, term, &global_env, None);
                writeln!(writer, "{name} definido.")?;
                writeln!(writer, "==> Val: {value:?}")?;
                writeln!(writer, "    Typ: {term_ty:?}")?;

                global_names.push(name);
                global_env.push(value);
                global_types.push(term_ty);
            }
            Err(error) => writeln!(writer, "Error de sintaxis: {error}")?,
        }
    }
}

pub fn start_repl() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_repl(stdin.lock(), stdout.lock())
}

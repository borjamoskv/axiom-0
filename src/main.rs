#![forbid(unsafe_code)]
use micro_axiom_0::repl::start_repl;
use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    match start_repl() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "Error de entrada/salida: {error}");
            ExitCode::FAILURE
        }
    }
}

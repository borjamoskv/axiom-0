use micro_axiom_0::seqlock::SeqlockCell;
use std::{error::Error, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("No se pudo completar el ejemplo: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    // Cada lectura dispone de un máximo de ocho intentos, sin reintentos externos.
    const READ_BUDGET: usize = 8;
    let cell = SeqlockCell::<usize, 2>::new([10, 20]);
    let initial = cell.try_read(READ_BUDGET)?;
    println!(
        "Inicial: valor={:?}; versión={}; intentos={}",
        initial.value, initial.version, initial.attempts,
    );

    // La escritura sustituye el valor completo y devuelve la versión publicada.
    // Si hay contención o se agotan las versiones, propagamos el error y terminamos.
    let version = cell.try_write([21, 42])?;
    println!("Actualización publicada en la versión {version}");

    let snapshot = cell.try_read(READ_BUDGET)?;
    println!(
        "Actualizado: valor={:?}; versión={}; intentos={}",
        snapshot.value, snapshot.version, snapshot.attempts,
    );

    // Un presupuesto de cero permite observar un error de lectura de forma
    // determinista. La aplicación decide cómo tratarlo; aquí solo lo mostramos.
    match cell.try_read(0) {
        Err(error) => println!("Lectura sin presupuesto: {error}"),
        Ok(_) => return Err("una lectura sin presupuesto no debe completarse".into()),
    }
    Ok(())
}

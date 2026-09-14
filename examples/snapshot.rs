use micro_axiom_0::seqlock::SeqlockCell;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cell = SeqlockCell::<usize, 2>::new([0, 0]);
    std::thread::scope(|scope| {
        scope
            .spawn(|| cell.try_write([21, 42]))
            .join()
            .expect("writer panicked")
    })?;
    let snapshot = cell.try_read(8)?;
    println!(
        "snapshot: {:?}; version={}; attempts={}",
        snapshot.value, snapshot.version, snapshot.attempts,
    );
    Ok(())
}

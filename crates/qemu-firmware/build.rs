use std::{env, error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let script = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("link.ld");
    println!("cargo:rustc-link-arg=-T{}", script.display());
    println!("cargo:rerun-if-changed=link.ld");
    println!("cargo:rerun-if-changed=src/boot.S");
    Ok(())
}

use std::{env, error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let linker_script = fs::read_to_string("utrace_linker.x.in")?;

    let out = &PathBuf::from(env::var("OUT_DIR")?);
    fs::write(out.join("utrace_linker.x"), linker_script)?;

    println!("cargo:rustc-link-search={}", out.display());

    Ok(())
}

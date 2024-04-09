use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
    elf_file: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    utrace_parser::elf_parser::parse(args.elf_file)?;

    Ok(())
}

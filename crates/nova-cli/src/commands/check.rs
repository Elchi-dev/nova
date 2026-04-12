use colored::Colorize;
use std::path::PathBuf;

pub fn execute(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("{} {}", "checking".green().bold(), path.display());

    // TODO: Run lexer + parser + type checker without codegen
    println!(
        "{}",
        "note: check not yet implemented".yellow().dimmed()
    );

    Ok(())
}

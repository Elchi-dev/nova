use colored::Colorize;
use std::path::PathBuf;

pub fn execute(path: PathBuf, _open: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("{} {}", "documenting".green().bold(), path.display());
    println!(
        "{}",
        "note: doc generator not yet implemented".yellow().dimmed()
    );
    Ok(())
}

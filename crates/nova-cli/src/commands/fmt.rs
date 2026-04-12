use colored::Colorize;
use std::path::PathBuf;

pub fn execute(path: PathBuf, check: bool) -> Result<(), Box<dyn std::error::Error>> {
    let action = if check { "checking format" } else { "formatting" };
    println!("{} {}", action.green().bold(), path.display());
    println!(
        "{}",
        "note: formatter not yet implemented".yellow().dimmed()
    );
    Ok(())
}

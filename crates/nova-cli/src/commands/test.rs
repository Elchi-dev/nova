use colored::Colorize;
use std::path::PathBuf;

pub fn execute(path: PathBuf, filter: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("{} {}", "testing".green().bold(), path.display());
    if let Some(f) = &filter {
        println!("{} filter: {}", "→".dimmed(), f.cyan());
    }
    println!(
        "{}",
        "note: test runner not yet implemented".yellow().dimmed()
    );
    Ok(())
}

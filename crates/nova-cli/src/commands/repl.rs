use colored::Colorize;

pub fn execute() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Nova REPL v0.1.0".cyan().bold());
    println!("Type {} to exit\n", ":quit".dimmed());
    println!(
        "{}",
        "note: REPL not yet implemented".yellow().dimmed()
    );
    Ok(())
}

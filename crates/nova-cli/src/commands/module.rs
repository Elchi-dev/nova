use colored::Colorize;

pub fn add(name: String, version: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let ver = version.as_deref().unwrap_or("latest");
    println!(
        "{} {} @ {}",
        "adding".green().bold(),
        name.cyan(),
        ver.dimmed()
    );
    println!(
        "{}",
        "note: package manager not yet implemented".yellow().dimmed()
    );
    Ok(())
}

pub fn remove(name: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("{} {}", "removing".green().bold(), name.cyan());
    println!(
        "{}",
        "note: package manager not yet implemented".yellow().dimmed()
    );
    Ok(())
}

pub fn update() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "updating dependencies".green().bold());
    println!(
        "{}",
        "note: package manager not yet implemented".yellow().dimmed()
    );
    Ok(())
}

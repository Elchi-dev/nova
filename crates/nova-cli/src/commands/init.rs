use colored::Colorize;

pub fn execute(name: String, lib: bool) -> Result<(), Box<dyn std::error::Error>> {
    let kind = if lib { "library" } else { "binary" };
    println!(
        "{} new Nova {} project: {}",
        "creating".green().bold(),
        kind,
        name.cyan()
    );
    println!(
        "{}",
        "note: project scaffolding not yet implemented"
            .yellow()
            .dimmed()
    );
    Ok(())
}

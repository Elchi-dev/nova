use colored::Colorize;
use std::path::PathBuf;

pub fn execute(
    path: PathBuf,
    output: Option<String>,
    release: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mode = if release { "release" } else { "debug" };
    let out_name = output.unwrap_or_else(|| "output".to_string());

    println!(
        "{} {} ({})",
        "building".green().bold(),
        path.display(),
        mode
    );
    println!("{} build target: {}", "→".dimmed(), out_name.cyan());

    // TODO: Full compilation pipeline → LLVM IR → binary
    println!(
        "{}",
        "note: build pipeline not yet implemented".yellow().dimmed()
    );

    Ok(())
}

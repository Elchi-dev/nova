use colored::Colorize;
use std::path::PathBuf;

pub fn execute(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let file = if path.is_dir() {
        // Look for main.nova or main.nv in directory
        let main_nova = path.join("main.nova");
        let main_nv = path.join("main.nv");
        if main_nova.exists() {
            main_nova
        } else if main_nv.exists() {
            main_nv
        } else {
            return Err("no main.nova or main.nv found in directory".into());
        }
    } else {
        path.clone()
    };

    if !file.exists() {
        return Err(format!("file not found: {}", file.display()).into());
    }

    let source = std::fs::read_to_string(&file)?;
    println!(
        "{} {}",
        "checking".green().bold(),
        file.display().to_string().dimmed()
    );

    // Lexer
    let tokens = nova_compiler::lexer::tokenize(&source)?;

    // Parser
    let program = nova_compiler::parser::parse(tokens)?;

    // Type checker
    let result = nova_compiler::typechecker::check(&program);

    if result.errors.is_empty() {
        println!(
            "  {} no errors found",
            "✓".green().bold()
        );
    } else {
        for err in &result.errors {
            println!(
                "  {} {}",
                "✗".red().bold(),
                err
            );
        }
        return Err(format!(
            "found {} type error(s)",
            result.errors.len()
        )
        .into());
    }

    if !result.warnings.is_empty() {
        for warn in &result.warnings {
            println!(
                "  {} {}",
                "⚠".yellow().bold(),
                warn
            );
        }
    }

    Ok(())
}

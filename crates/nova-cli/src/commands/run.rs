use colored::Colorize;
use std::path::PathBuf;

pub fn execute(file: PathBuf, _args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("file not found: {}", file.display()).into());
    }

    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext != "nova" {
        return Err(format!("expected a .nova file, got .{ext}").into());
    }

    let source = std::fs::read_to_string(&file)?;
    println!(
        "{} {}",
        "compiling".green().bold(),
        file.display().to_string().dimmed()
    );

    // Lexer → Parser → Type Check → Codegen → Execute
    let tokens = nova_compiler::lexer::tokenize(&source)?;
    let _ast = nova_compiler::parser::parse(tokens)?;

    println!(
        "{} parsed {} tokens successfully",
        "✓".green().bold(),
        source.len()
    );

    // TODO: Type checking, codegen, execution
    println!(
        "{}",
        "note: execution not yet implemented — compiler frontend only"
            .yellow()
            .dimmed()
    );

    Ok(())
}

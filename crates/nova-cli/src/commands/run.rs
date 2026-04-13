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
    let ast = nova_compiler::parser::parse(tokens)?;

    // Type check
    let result = nova_compiler::typechecker::check(&ast);

    if !result.errors.is_empty() {
        for err in &result.errors {
            eprintln!(
                "  {} {}",
                "✗".red().bold(),
                err
            );
        }
        return Err(format!("found {} type error(s)", result.errors.len()).into());
    }

    println!(
        "{} type-checked {} statement(s) successfully",
        "✓".green().bold(),
        ast.statements.len()
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

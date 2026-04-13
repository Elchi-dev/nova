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

    // Lexer
    let tokens = nova_compiler::lexer::tokenize(&source)?;

    // Parser
    let ast = nova_compiler::parser::parse(tokens)?;

    // Type check
    let check_result = nova_compiler::typechecker::check(&ast);
    if !check_result.errors.is_empty() {
        for err in &check_result.errors {
            eprintln!("  {} {}", "✗".red().bold(), err);
        }
        return Err(format!("found {} type error(s)", check_result.errors.len()).into());
    }

    // Execute
    let output = nova_compiler::interpreter::run(&ast).map_err(|e| {
        format!("{}", e)
    })?;

    // Print output
    for line in &output {
        println!("{line}");
    }

    Ok(())
}

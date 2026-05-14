use colored::Colorize;
use nova_codegen::Codegen;
use nova_compiler::{lexer, parser, typechecker};
use std::path::PathBuf;
use std::process::Command;

pub fn execute(
    path: PathBuf,
    output: Option<String>,
    release: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mode = if release { "release" } else { "debug" };
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let out_name = output.unwrap_or_else(|| stem.to_string());

    println!(
        "{} {} ({})",
        "building".green().bold(),
        path.display(),
        mode
    );

    let source = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read '{}': {e}", path.display()))?;

    let tokens = lexer::tokenize(&source).map_err(|e| format!("lex error: {e}"))?;

    let ast = parser::parse(tokens).map_err(|e| format!("parse error: {e}"))?;

    let check_result = typechecker::check(&ast);
    if !check_result.errors.is_empty() {
        for e in &check_result.errors {
            eprintln!("  {} {e}", "type error:".red().bold());
        }
        return Err("type checking failed".into());
    }

    print!("  {} LLVM IR ... ", "generating".dimmed());
    let context = nova_codegen::inkwell::context::Context::create();
    let mut codegen = Codegen::new(&context, &stem);
    match codegen.compile_program(&ast) {
        Ok(ir) => {
            println!("{}", "ok".green());
            if std::env::var("NOVA_DUMP_IR").is_ok() {
                println!(
                    "\n{}\n{ir}\n{}",
                    "-- LLVM IR --".dimmed(),
                    "-------------".dimmed()
                );
            }
        }
        Err(e) => {
            println!("{}", "failed".red());
            return Err(format!("codegen error: {e}").into());
        }
    }

    let obj_path = format!("/tmp/{stem}.o");
    print!("  {} object file ... ", "emitting".dimmed());
    codegen.emit_object_file(&obj_path)?;
    println!("{}", "ok".green());

    print!("  {} binary ... ", "linking".dimmed());
    let status = Command::new("cc")
        .args([&obj_path, "-o", &out_name])
        .status()
        .map_err(|e| format!("linker not found: {e}"))?;

    if !status.success() {
        println!("{}", "failed".red());
        return Err("linker returned non-zero exit code".into());
    }
    println!("{}", "ok".green());
    std::fs::remove_file(&obj_path).ok();

    println!("\n  {} {}", "->".green().bold(), out_name.cyan().bold());
    Ok(())
}

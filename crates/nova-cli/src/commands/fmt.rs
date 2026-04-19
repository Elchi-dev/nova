use colored::Colorize;
use std::path::PathBuf;

pub fn execute(path: PathBuf, check: bool) -> Result<(), Box<dyn std::error::Error>> {
    let files = collect_nova_files(&path)?;

    if files.is_empty() {
        println!("{}", "no .nova or .nv files found".yellow().dimmed());
        return Ok(());
    }

    let mut unformatted_count = 0;

    for file in &files {
        let source = std::fs::read_to_string(file)?;
        let tokens = nova_compiler::lexer::tokenize(&source)?;
        let program = nova_compiler::parser::parse(tokens)?;
        let formatted = nova_compiler::formatter::format(&program);

        if source != formatted {
            if check {
                println!("  {} {}", "✗".red(), file.display());
                unformatted_count += 1;
            } else {
                std::fs::write(file, &formatted)?;
                println!("  {} {}", "✓".green(), file.display());
            }
        } else if !check {
            println!(
                "  {} {} {}",
                "✓".green(),
                file.display(),
                "(unchanged)".dimmed()
            );
        }
    }

    if check && unformatted_count > 0 {
        return Err(format!("{unformatted_count} file(s) need formatting — run `nova fmt`").into());
    }

    if !check {
        println!("\n{} formatted {} file(s)", "✓".green().bold(), files.len());
    }

    Ok(())
}

fn collect_nova_files(path: &PathBuf) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();

    if path.is_file() {
        files.push(path.clone());
    } else if path.is_dir() {
        collect_recursive(path, &mut files)?;
    }

    Ok(files)
}

fn collect_recursive(
    dir: &PathBuf,
    files: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_recursive(&path, files)?;
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && (ext == "nova" || ext == "nv")
        {
            files.push(path);
        }
    }
    Ok(())
}

use colored::Colorize;
use std::path::PathBuf;
use std::time::Instant;

pub fn execute(path: PathBuf, filter: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let files = collect_test_files(&path)?;

    if files.is_empty() {
        println!("{}", "no test files found".yellow().dimmed());
        return Ok(());
    }

    let mut total = 0;
    let mut passed = 0;
    let mut failed = 0;
    let mut failures: Vec<(String, String)> = Vec::new();
    let start = Instant::now();

    for file in &files {
        let source = std::fs::read_to_string(file)?;
        let tokens = nova_compiler::lexer::tokenize(&source)?;
        let program = nova_compiler::parser::parse(tokens)?;

        // Find all test functions (named test_*)
        let test_fns: Vec<String> = program
            .statements
            .iter()
            .filter_map(|stmt| {
                if let nova_compiler::ast::Statement::FunctionDef { name, .. } = stmt {
                    if name.starts_with("test_") {
                        // Apply name filter if provided
                        if let Some(ref f) = filter {
                            if !name.contains(f.as_str()) {
                                return None;
                            }
                        }
                        return Some(name.clone());
                    }
                }
                None
            })
            .collect();

        if test_fns.is_empty() {
            continue;
        }

        let file_name = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        println!("\n{} {}", "running".dimmed(), file.display().to_string().dimmed());

        for test_name in &test_fns {
            total += 1;

            // Create source that calls the test function
            let test_source = format!("{source}\n{test_name}()");
            let test_tokens = nova_compiler::lexer::tokenize(&test_source)?;
            let test_program = nova_compiler::parser::parse(test_tokens)?;

            match nova_compiler::interpreter::run(&test_program) {
                Ok(_) => {
                    passed += 1;
                    println!("  {} {file_name}::{test_name}", "✓".green());
                }
                Err(e) => {
                    failed += 1;
                    let err_msg = e.to_string();
                    println!("  {} {file_name}::{test_name} — {}", "✗".red(), err_msg.red());
                    failures.push((format!("{file_name}::{test_name}"), err_msg));
                }
            }
        }
    }

    let elapsed = start.elapsed();
    println!();

    if failed == 0 {
        println!(
            "{} {} test(s) passed in {:.2}s",
            "✓".green().bold(),
            total,
            elapsed.as_secs_f64()
        );
    } else {
        if !failures.is_empty() {
            println!("{}", "failures:".red().bold());
            for (name, err) in &failures {
                println!("  {name}: {err}");
            }
            println!();
        }
        println!(
            "{} {passed} passed, {failed} failed out of {total} test(s) in {:.2}s",
            "✗".red().bold(),
            elapsed.as_secs_f64()
        );
        return Err(format!("{failed} test(s) failed").into());
    }

    Ok(())
}

fn collect_test_files(path: &PathBuf) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
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
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext == "nova" || ext == "nv" {
                // Look for files with test_ functions or in a tests/ dir
                let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let in_tests_dir = path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .map(|n| n == "tests")
                    .unwrap_or(false);

                if name.starts_with("test_") || name.ends_with("_test") || in_tests_dir {
                    files.push(path);
                } else {
                    // Also check file content for test_ functions
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if content.contains("fn test_") {
                            files.push(path);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

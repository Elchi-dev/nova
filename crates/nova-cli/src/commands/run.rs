use colored::Colorize;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

pub fn execute(
    file: PathBuf,
    _args: Vec<String>,
    watch: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("file not found: {}", file.display()).into());
    }

    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext != "nova" && ext != "nv" {
        return Err(format!("expected a .nova or .nv file, got .{ext}").into());
    }

    // Initial run
    run_once(&file);

    if !watch {
        return Ok(());
    }

    // Watch mode
    println!(
        "\n{} watching {} for changes (Ctrl+C to exit)",
        "✦".cyan().bold(),
        file.display().to_string().dimmed()
    );

    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    // Watch the file's directory (notify fires on parent dir changes)
    let watch_path = file.parent().unwrap_or(Path::new("."));
    watcher.watch(watch_path, RecursiveMode::NonRecursive)?;

    // Debounce rapid fires
    let mut last_run = std::time::Instant::now();

    for res in rx {
        match res {
            Ok(Event { kind, paths, .. }) => {
                if !matches!(kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    continue;
                }

                // Only react to our file (or any .nova/.nv in same dir)
                let relevant = paths.iter().any(|p| {
                    p == &file
                        || p.extension()
                            .and_then(|e| e.to_str())
                            .map(|e| e == "nova" || e == "nv")
                            .unwrap_or(false)
                });

                if !relevant {
                    continue;
                }

                // Debounce: skip if less than 100ms since last run
                if last_run.elapsed() < Duration::from_millis(100) {
                    continue;
                }
                last_run = std::time::Instant::now();

                println!("\n{}", "─ change detected ─".dimmed());
                run_once(&file);
                println!("\n{} still watching (Ctrl+C to exit)", "✦".cyan().bold());
            }
            Err(e) => eprintln!("watch error: {e}"),
        }
    }

    Ok(())
}

fn run_once(file: &Path) {
    match execute_file(file) {
        Ok(_) => {}
        Err(e) => eprintln!("  {} {}", "✗".red().bold(), e),
    }
}

fn execute_file(file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(file)?;

    let tokens = nova_compiler::lexer::tokenize(&source)?;
    let ast = nova_compiler::parser::parse(tokens)?;

    let check_result = nova_compiler::typechecker::check(&ast);
    if !check_result.errors.is_empty() {
        for err in &check_result.errors {
            eprintln!("  {} {}", "✗".red().bold(), err);
        }
        return Err(format!("found {} type error(s)", check_result.errors.len()).into());
    }

    let output = nova_compiler::interpreter::run(&ast).map_err(|e| format!("{e}"))?;

    for line in &output {
        println!("{line}");
    }

    Ok(())
}

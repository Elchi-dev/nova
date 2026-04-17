use colored::Colorize;
use std::io::{self, BufRead, Write};

pub fn execute() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Nova REPL v0.1.1".cyan().bold());
    println!(
        "Type {} for commands, {} to exit\n",
        ":help".dimmed(),
        ":quit".dimmed()
    );

    let mut interpreter = nova_compiler::interpreter::eval::Interpreter::new();
    let mut line_buffer = String::new();
    let mut in_block = false;
    let stdin = io::stdin();

    loop {
        // Prompt
        let prompt = if in_block {
            format!("{} ", "...".dimmed())
        } else {
            format!("{} ", ">>>".green().bold())
        };
        print!("{prompt}");
        io::stdout().flush()?;

        // Read line
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            // EOF
            println!();
            break;
        }
        let line = line.trim_end_matches('\n').trim_end_matches('\r');

        // Handle REPL commands
        if !in_block {
            match line.trim() {
                ":quit" | ":q" | ":exit" => break,
                ":help" | ":h" => {
                    print_help();
                    continue;
                }
                ":clear" | ":c" => {
                    print!("\x1B[2J\x1B[H"); // ANSI clear screen
                    io::stdout().flush()?;
                    continue;
                }
                "" => continue,
                _ => {}
            }
        }

        // Multi-line block detection
        if line.trim().ends_with(':') && !in_block {
            in_block = true;
            line_buffer = line.to_string();
            line_buffer.push('\n');
            continue;
        }

        if in_block {
            if line.trim().is_empty() {
                // Empty line ends the block
                in_block = false;
            } else {
                line_buffer.push_str(line);
                line_buffer.push('\n');
                continue;
            }
        } else {
            line_buffer = line.to_string();
        }

        // Skip empty input
        if line_buffer.trim().is_empty() {
            line_buffer.clear();
            continue;
        }

        // Try to evaluate
        eval_input(&mut interpreter, &line_buffer);
        line_buffer.clear();
    }

    Ok(())
}

fn eval_input(interpreter: &mut nova_compiler::interpreter::eval::Interpreter, input: &str) {
    // Lex
    let tokens = match nova_compiler::lexer::tokenize(input) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("  {} {}", "syntax error:".red().bold(), e);
            return;
        }
    };

    // Parse
    let program = match nova_compiler::parser::parse(tokens) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("  {} {}", "parse error:".red().bold(), e);
            return;
        }
    };

    // Execute (skip type checking in REPL for faster feedback)
    match interpreter.execute_repl(&program) {
        Ok(Some(val)) => {
            // Print the result of an expression
            println!("{}", format!("{val}").cyan());
        }
        Ok(None) => {
            // Statement executed, print any output
        }
        Err(e) => {
            eprintln!("  {} {}", "runtime error:".red().bold(), e);
        }
    }

    // Print any captured output
    let output = std::mem::take(&mut interpreter.output);
    for line in output {
        println!("{line}");
    }
}

fn print_help() {
    println!("{}", "Nova REPL Commands:".yellow().bold());
    println!("  {}  — Exit the REPL", ":quit, :q".dimmed());
    println!("  {}  — Show this help", ":help, :h".dimmed());
    println!("  {} — Clear the screen", ":clear, :c".dimmed());
    println!();
    println!("{}", "Tips:".yellow().bold());
    println!("  • Expressions are evaluated and their result is displayed");
    println!("  • Lines ending with : start a block (end with empty line)");
    println!("  • Variables and functions persist across lines");
    println!();
}

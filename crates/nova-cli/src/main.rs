use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

/// Nova — a fast, ergonomic systems language with first-class distributed primitives.
#[derive(Parser)]
#[command(
    name = "nova",
    version,
    about = "The Nova language toolchain",
    long_about = None,
    propagate_version = true,
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile a Nova source file to a native binary
    Build {
        /// Source file (.nv)
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Output binary path
        #[arg(short, long, value_name = "OUT")]
        output: Option<PathBuf>,

        /// Emit LLVM IR instead of a binary (debug)
        #[arg(long)]
        emit_ir: bool,
    },

    /// Compile and immediately run a Nova program
    Run {
        /// Source file (.nv)
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Arguments passed to the program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Check a Nova file for errors without producing output
    Check {
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    /// Format Nova source files
    Fmt {
        /// Files or directories to format
        #[arg(value_name = "PATH", default_value = ".")]
        paths: Vec<PathBuf>,

        /// Check formatting without modifying files
        #[arg(long)]
        check: bool,
    },

    /// Lint Nova source files
    Lint {
        #[arg(value_name = "PATH", default_value = ".")]
        paths: Vec<PathBuf>,
    },

    /// Manage packages (add, remove, update)
    Pkg {
        #[command(subcommand)]
        action: PkgAction,
    },

    /// Create a new Nova project
    New {
        /// Project name
        name: String,

        /// Create a library project instead of a binary
        #[arg(long)]
        lib: bool,
    },

    /// Show version information
    Version,
}

#[derive(Subcommand)]
enum PkgAction {
    /// Add a dependency
    Add { name: String },
    /// Remove a dependency
    Remove { name: String },
    /// Update all dependencies
    Update,
    /// List installed packages
    List,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Build { file, output, emit_ir } => cmd_build(file, output, emit_ir),
        Command::Run { file, args } => cmd_run(file, args),
        Command::Check { file } => cmd_check(file),
        Command::Fmt { paths, check } => cmd_fmt(paths, check),
        Command::Lint { paths } => cmd_lint(paths),
        Command::Pkg { action } => cmd_pkg(action),
        Command::New { name, lib } => cmd_new(name, lib),
        Command::Version => cmd_version(),
    }
}

fn cmd_build(file: PathBuf, output: Option<PathBuf>, emit_ir: bool) {
    let output = output.unwrap_or_else(|| {
        file.with_extension("")
    });

    print_step("Compiling", &file.display().to_string());

    let result = nova_compiler::compile(&file, &output);

    if result.errors.is_empty() {
        println!(
            "  {} {} → {}",
            "Built".green().bold(),
            file.display(),
            output.display()
        );
    } else {
        for err in &result.errors {
            eprintln!("  {} {}", "error:".red().bold(), err);
        }
        std::process::exit(1);
    }

    let _ = emit_ir; // TODO: implement IR emission
}

fn cmd_run(file: PathBuf, args: Vec<String>) {
    print_step("Running", &file.display().to_string());
    // TODO: compile to temp dir then exec
    println!("{}", "nova run: not yet implemented".yellow());
    let _ = args;
}

fn cmd_check(file: PathBuf) {
    print_step("Checking", &file.display().to_string());
    let tmp = file.with_extension("_check_tmp");
    let result = nova_compiler::compile(&file, &tmp);

    if result.errors.is_empty() {
        println!("  {} no errors found", "ok".green().bold());
    } else {
        for err in &result.errors {
            eprintln!("  {} {}", "error:".red().bold(), err);
        }
        std::process::exit(1);
    }
}

fn cmd_fmt(paths: Vec<PathBuf>, check: bool) {
    print_step("Formatting", &format!("{} path(s)", paths.len()));
    // TODO: implement formatter
    println!("{}", "nova fmt: not yet implemented".yellow());
    let _ = check;
}

fn cmd_lint(paths: Vec<PathBuf>) {
    print_step("Linting", &format!("{} path(s)", paths.len()));
    // TODO: implement linter
    println!("{}", "nova lint: not yet implemented".yellow());
}

fn cmd_pkg(action: PkgAction) {
    match action {
        PkgAction::Add { name } => println!("Adding package: {} (not yet implemented)", name),
        PkgAction::Remove { name } => println!("Removing package: {} (not yet implemented)", name),
        PkgAction::Update => println!("Updating packages (not yet implemented)"),
        PkgAction::List => println!("Listing packages (not yet implemented)"),
    }
}

fn cmd_new(name: String, lib: bool) {
    let kind = if lib { "library" } else { "binary" };
    print_step("Creating", &format!("{} project '{}'", kind, name));
    // TODO: scaffold project directory
    println!("{}", "nova new: not yet implemented".yellow());
}

fn cmd_version() {
    println!(
        "{} {}  {}",
        "Nova".cyan().bold(),
        env!("CARGO_PKG_VERSION"),
        "https://github.com/Elchi-dev/nova".dimmed()
    );
}

fn print_step(verb: &str, target: &str) {
    println!("  {:>12} {}", verb.cyan().bold(), target);
}

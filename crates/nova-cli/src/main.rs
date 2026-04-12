use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

mod commands;

#[derive(Parser)]
#[command(
    name = "nova",
    about = "The Nova Programming Language",
    long_about = "Nova — Fast, modular, Python-like syntax with arena-based memory and hot-reloading.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a Nova source file directly
    Run {
        /// Path to the .nova file
        file: PathBuf,

        /// Arguments passed to the program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Compile a Nova project to a binary
    Build {
        /// Path to the project or file
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output binary name
        #[arg(short, long)]
        output: Option<String>,

        /// Build in release mode with optimizations
        #[arg(long)]
        release: bool,
    },

    /// Type-check and lint without compiling
    Check {
        /// Path to check
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Format Nova source files
    Fmt {
        /// Path to format
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Check formatting without writing changes
        #[arg(long)]
        check: bool,
    },

    /// Run tests
    Test {
        /// Path to test directory or file
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Filter tests by name
        #[arg(short, long)]
        filter: Option<String>,
    },

    /// Generate documentation
    Doc {
        /// Path to document
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Open docs in browser after generation
        #[arg(long)]
        open: bool,
    },

    /// Start an interactive REPL
    Repl,

    /// Initialize a new Nova project
    Init {
        /// Project name
        name: String,

        /// Use library template instead of binary
        #[arg(long)]
        lib: bool,
    },

    /// Manage dependencies
    Mod {
        #[command(subcommand)]
        action: ModCommands,
    },
}

#[derive(Subcommand)]
enum ModCommands {
    /// Add a dependency
    Add {
        /// Package name
        name: String,
        /// Version constraint
        #[arg(short, long)]
        version: Option<String>,
    },
    /// Remove a dependency
    Remove {
        /// Package name
        name: String,
    },
    /// Update dependencies
    Update,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Run { file, args } => commands::run::execute(file, args),
        Commands::Build {
            path,
            output,
            release,
        } => commands::build::execute(path, output, release),
        Commands::Check { path } => commands::check::execute(path),
        Commands::Fmt { path, check } => commands::fmt::execute(path, check),
        Commands::Test { path, filter } => commands::test::execute(path, filter),
        Commands::Doc { path, open } => commands::doc::execute(path, open),
        Commands::Repl => commands::repl::execute(),
        Commands::Init { name, lib } => commands::init::execute(name, lib),
        Commands::Mod { action } => match action {
            ModCommands::Add { name, version } => commands::module::add(name, version),
            ModCommands::Remove { name } => commands::module::remove(name),
            ModCommands::Update => commands::module::update(),
        },
    };

    if let Err(e) = result {
        eprintln!("{} {}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

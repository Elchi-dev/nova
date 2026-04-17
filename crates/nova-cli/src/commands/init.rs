use colored::Colorize;
use std::fs;
use std::path::Path;

pub fn execute(name: String, lib: bool) -> Result<(), Box<dyn std::error::Error>> {
    let project_dir = Path::new(&name);

    if project_dir.exists() {
        return Err(format!("directory `{name}` already exists").into());
    }

    // Create directory structure
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("tests"))?;

    // Create nova.toml manifest
    let manifest = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
description = ""

[dependencies]
"#
    );
    fs::write(project_dir.join("nova.toml"), manifest)?;

    // Create main source file
    if lib {
        let lib_source = r#"# Nova library

pub fn hello(name: str) -> str:
    return "Hello, " + name + "!"
"#;
        fs::write(project_dir.join("src").join("lib.nova"), lib_source)?;

        let test_source = format!(
            r#"# Tests for {name}

import src.lib

fn test_hello():
    let result = lib.hello("World")
    print("Testing hello: " + result)
"#
        );
        fs::write(project_dir.join("tests").join("test_lib.nova"), test_source)?;
    } else {
        let main_source = r#"# Nova application

fn main():
    print("Hello, Nova!")
"#;
        fs::write(project_dir.join("src").join("main.nova"), main_source)?;

        let test_source = format!(
            r#"# Tests for {name}

fn test_basic():
    let x = 1 + 1
    print("1 + 1 = " + str(x))
"#
        );
        fs::write(project_dir.join("tests").join("test_main.nova"), test_source)?;
    }

    // Create .gitignore
    let gitignore = r#"/build
/dist
*.pyc
__pycache__
"#;
    fs::write(project_dir.join(".gitignore"), gitignore)?;

    // Create README
    let readme = format!(
        r#"# {name}

A Nova project.

## Getting Started

```bash
nova run src/main.nova
nova test
nova check
```
"#
    );
    fs::write(project_dir.join("README.md"), readme)?;

    // Print success
    let kind = if lib { "library" } else { "application" };
    println!(
        "{} Nova {kind} `{}`",
        "created".green().bold(),
        name.cyan()
    );
    println!();

    let tree = if lib {
        format!(
            "  {name}/\n  ├── nova.toml\n  ├── README.md\n  ├── .gitignore\n  ├── src/\n  │   └── lib.nova\n  └── tests/\n      └── test_lib.nova"
        )
    } else {
        format!(
            "  {name}/\n  ├── nova.toml\n  ├── README.md\n  ├── .gitignore\n  ├── src/\n  │   └── main.nova\n  └── tests/\n      └── test_main.nova"
        )
    };
    println!("{tree}");
    println!();
    println!("  Run: {} {}", "nova run".cyan(), format!("{name}/src/main.nova").dimmed());

    Ok(())
}

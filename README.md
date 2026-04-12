<p align="center">
  <h1 align="center">✦ Nova</h1>
  <p align="center"><strong>Fast. Modular. Familiar.</strong></p>
  <p align="center">A compiled programming language with Python-like syntax, arena-based memory, and transparent hot-reloading.</p>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#quickstart">Quickstart</a> •
  <a href="#syntax">Syntax</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#building">Building</a>
</p>

---

## Why Nova?

Nova is built on a simple idea: **writing fast software shouldn't be painful.**

Python is beautiful to write but slow. Rust is fast but has a steep learning curve. Go is simple but lacks expressiveness. Nova takes the best parts of each — Python's readability, Rust's performance, Go's simplicity — and combines them with features no other language offers.

## Features

- **Context-Aware Memory** — No garbage collector. No borrow checker. Arena-based allocation with compiler-driven escape analysis. Objects are freed instantly when their scope ends, in bulk, with zero runtime overhead.
- **Transparent Hot-Reloading** — Your code is automatically split into modules at compile time. Change a file, and only the affected module is reloaded — while the program keeps running. No manual setup required.
- **Python-Like Syntax** — Tab-based indentation by default, with optional `{}` for one-liners. If you know Python, you can read Nova.
- **Direct C Interop** — Import C headers directly: `import foreign("raylib.h", lang: "c")`. No bindings, no wrappers.
- **Built-in Decorators** — Compile-time code transformation via `@decorators`, more powerful than Python's runtime decorators.
- **Effect System** — Functions declare their side effects. Pure functions are guaranteed pure by the compiler.
- **Design by Contract** — Built-in `require` / `ensure` for pre/post-conditions, verified at compile time where possible.
- **Pipe Operator** — Chain operations cleanly: `data |> filter |> transform |> output`.
- **Semantic-Aware Compilation** — The compiler understands intent, not just syntax. `sort |> reverse` becomes a single reverse-sort pass automatically.
- **Structured Concurrency** — Scope-based tasks that can never be orphaned, integrated with the arena memory model.

## Quickstart

```bash
# Build from source
git clone https://github.com/Elchi-dev/nova.git
cd nova
cargo build --release

# Run a Nova file
nova run examples/showcase.nova

# Other commands
nova build .              # Compile to binary
nova check .              # Type-check and lint
nova fmt .                # Format source files
nova test .               # Run tests
nova repl                 # Interactive REPL
nova init my_project      # Scaffold new project
```

## Syntax

```python
import std.io

# Structs with defaults
pub struct Player:
    let name: str
    let mut health: int = 100

# Contracts
fn divide(a: float, b: float) -> float:
    require b != 0.0
    ensure result * b == a
    return a / b

# Decorators + pipe operator
@cached
fn process(data: list[int]) -> list[int]:
    return data
        |> filter(x => x > 0)
        |> map(x => x * 2)
        |> sort

# Inline brace syntax
fn double(x: int) -> int { return x * 2; }

# Effect system
pure fn add(a: int, b: int) -> int:
    return a + b

fn read_file(path: str) -> str [io, error]:
    return io.read(path)

# Direct C interop
import foreign("raylib.h", lang: "c")

fn main():
    let player = Player { name: "Nova", health: 100 }
    io.print(f"Hello, {player.name}!")
```

## Architecture

Nova is structured as a Rust workspace with three crates:

```
nova/
├── crates/
│   ├── nova-cli/          # CLI tool (nova run, build, check, fmt, ...)
│   ├── nova-compiler/     # Lexer → Parser → Type Checker → Codegen
│   │   └── src/
│   │       ├── lexer/     # Tokenization with indentation tracking
│   │       ├── parser/    # Recursive descent parser → AST
│   │       ├── ast/       # Abstract syntax tree definitions
│   │       ├── typechecker/  # Static type inference & checking
│   │       ├── semantic/  # Escape analysis, module splitting, optimizations
│   │       └── codegen/   # LLVM IR generation
│   └── nova-runtime/      # Runtime support
│       └── src/
│           ├── memory/    # Arena allocator (Context-Aware Memory)
│           ├── module/    # Hot-reload module manager
│           └── ffi/       # C interop layer
├── examples/              # Example .nova files
└── docs/                  # Language documentation
```

## Building

Requires Rust 1.75+ and Cargo.

```bash
cargo build              # Debug build
cargo build --release    # Optimized build
cargo test               # Run all tests
```

## Status

Nova is in early development. The lexer and parser are functional, the arena memory system is implemented, and the module hot-reload manager is in place. Active work is focused on the type checker and LLVM codegen.

## License

MIT — see [LICENSE](LICENSE).

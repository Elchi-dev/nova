<p align="center">
  <a href="https://github.com/Elchi-dev/nova/actions/workflows/ci.yml"><img src="https://github.com/Elchi-dev/nova/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <img src="https://img.shields.io/badge/status-pre--alpha-orange" alt="Status">
  <img src="https://img.shields.io/badge/language-Rust-B7410E" alt="Language">
  <img src="https://img.shields.io/badge/license-MIT-blue" alt="License">
</p>

<h1 align="center">✦ Nova</h1>
<p align="center"><strong>Write like Python. Run like Rust. Reload like Erlang.</strong></p>
<p align="center">A compiled programming language with arena-based memory,<br>transparent hot-reloading, and zero-overhead abstractions.</p>

<p align="center">
  <a href="#what-makes-nova-different">Why Nova?</a> •
  <a href="#quick-look">Quick Look</a> •
  <a href="#features-at-a-glance">Features</a> •
  <a href="#getting-started">Getting Started</a> •
  <a href="ROADMAP.md">Roadmap</a> •
  <a href="docs/NOVA_FEATURES.md">Full Feature Guide</a>
</p>

---

## What Makes Nova Different?

Every language makes you choose: **easy to write** *or* **fast to run**. Python is beautiful but slow. Rust is fast but complex. Go is simple but limited.

Nova rejects that trade-off.

| | Python | Go | Rust | **Nova** |
|---|--------|-----|------|----------|
| **Syntax** | Beautiful | Clean | Complex | **Beautiful** |
| **Performance** | ~100x slower than C | ~1.5x slower | C-speed | **C-speed (LLVM)** |
| **Memory** | GC (pauses) | GC (pauses) | Borrow checker (complex) | **Arenas (zero-cost)** |
| **Learning curve** | Days | Weeks | Months | **Days** |
| **Hot-reload** | ✗ | ✗ | ✗ | **✓ Built-in** |
| **Effect tracking** | ✗ | ✗ | ✗ | **✓ Built-in** |
| **C interop** | ctypes (painful) | CGo (limited) | FFI (manual) | **Header import** |

---

## Quick Look

```python
import std.io
import foreign("sqlite3.h", lang: "c")

pub struct User:
    let name: str
    let mut score: int = 0

# Effect system — the compiler knows this function does IO
fn load_user(id: int) -> User [io, error]:
    let row = db.query(f"SELECT * FROM users WHERE id = {id}")
    return User { name: row.name, score: row.score }

# Pure function — guaranteed no side effects, safe to cache
@cached
pure fn fibonacci(n: int) -> int:
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

# Contracts — compiler-verified preconditions
fn withdraw(account: Account, amount: float) -> Account:
    require amount > 0.0
    require amount <= account.balance
    ensure result.balance == account.balance - amount
    return Account { balance: account.balance - amount }

# Pipes — data flows left to right
fn process(data: list[int]) -> list[int]:
    return data
        |> filter(x => x > 0)
        |> map(x => x * 2)
        |> sort

# One-liners with braces
fn double(x: int) -> int { return x * 2; }

fn main():
    let user = load_user(1)
    let result = [1, 2, 3, 4, 5] |> process
    io.print(f"Hello {user.name}, result: {result}")
```

---

## Features at a Glance

### Context-Aware Memory
No garbage collector. No borrow checker. Nova uses arena-based allocation with compile-time escape analysis. ~95% of allocations are bump-allocated and freed in bulk at scope exit. The remaining ~5% (values that escape their scope) use lightweight reference counting. The developer writes normal code — the compiler decides.

### Transparent Hot-Reloading
The compiler automatically splits your code into modules based on dependency analysis. At runtime, changed modules are swapped using a blue-green strategy: the old version drains its active calls while the new version handles new ones. No manual module design. No `code_change` callbacks. Your code is live the moment you save.

### Effect System
Functions declare their side effects: `[io]`, `[error]`, `[net]`. The `pure` keyword guarantees zero side effects, enforced by the compiler. Pure functions can be cached, parallelized, and reordered safely.

### Design by Contract
Built-in `require` (preconditions) and `ensure` (postconditions) are verified at compile time where provable, and serve as runtime assertions in debug builds.

### Direct C Interop
`import foreign("header.h", lang: "c")` — the compiler parses the C header and generates type-safe bindings. No wrappers, no ceremony.

### Compile-Time Decorators
`@cached`, `@log_time`, `@validate` — decorators are compile-time code transformations, not runtime wrappers. Zero overhead from the decorator mechanism itself.

### Pipe Operator
`data |> transform |> filter |> output` — chain operations left-to-right instead of nesting function calls inside-out.

### Semantic-Aware Compilation
The compiler optimizes at the semantic level: `sort |> reverse` becomes a single reverse-sort. `map(f) |> map(g)` fuses into `map(f∘g)`. Same result, fewer operations. Opt out with `@literal`.

### Structured Concurrency
Scope-based tasks that cannot outlive their parent. Each task gets its own arena. No orphaned goroutines, no dangling futures.

---

## Getting Started

### Build from Source

```bash
git clone https://github.com/Elchi-dev/nova.git
cd nova
cargo build --release
```

### CLI

```bash
nova run file.nova        # Compile and execute (.nova or .nv)
nova run file.nova --watch # Re-run automatically on file changes
nova run script.nv        # Short extension also works
nova check                # Type-check + lint
nova fmt                  # Format source code (AST-based)
nova fmt --check          # Check formatting without writing
nova test                 # Auto-discover and run test_* functions
nova doc examples/        # Generate HTML documentation from ## doc comments
nova doc --open examples/ # Generate and open in browser
nova repl                 # Interactive REPL with persistent state
nova init my_project      # Scaffold new project with nova.toml, src/, tests/
nova init my_lib --lib    # Scaffold a library project
nova build                # Compile to native binary (in development)
nova mod add package      # Add dependency (in development)
```

---

## Architecture

```
nova/
├── crates/
│   ├── nova-cli/            # CLI binary — all commands in one tool
│   ├── nova-compiler/       # Compiler pipeline
│   │   ├── lexer/           #   Source → Tokens (with indentation)
│   │   ├── parser/          #   Tokens → AST (recursive descent)
│   │   ├── ast/             #   Node definitions for all constructs
│   │   ├── typechecker/     #   Static type inference & checking
│   │   ├── interpreter/     #   Tree-walking interpreter (nova run)
│   │   ├── semantic/        #   Escape analysis, effects, module splitting
│   │   └── codegen/         #   AST → LLVM IR → machine code
│   └── nova-runtime/        # Runtime support
│       ├── memory/          #   Arena allocator (Context-Aware Memory)
│       ├── module/          #   Hot-reload manager (blue-green swap)
│       └── ffi/             #   C interop layer
├── examples/                # Example .nova files
├── docs/                    # Language documentation
├── ROADMAP.md               # Detailed feature roadmap
└── LICENSE                  # MIT
```

---

## Known Challenges

We're building something ambitious. These are the hard problems we're actively solving:

**Inlining vs Hot-Reload** — LLVM inlining across module boundaries would break hot-reload. Our solution: dev mode uses indirect calls (module dispatch table), release mode enables full inlining. Cost: ~2-5ns per cross-module call in dev, zero in release.

**Struct Layout Changes** — Changing a struct's fields while the program runs would corrupt memory. Our solution: reject hot-reload when layouts change, with a clear error message. Behavior changes reload; shape changes require restart.

**Escape Analysis Awareness** — A small code change could flip an allocation from arena to ref-counted. Our solution: `nova check` reports allocation profiles, and warns when escape behavior changes.

See [ROADMAP.md](ROADMAP.md) for the full technical discussion.

---

## Status

Nova is in **pre-alpha**. The full developer workflow is in place — `nova init` scaffolds projects, `nova run` executes them, `nova check` type-checks, `nova fmt` formats, `nova test` runs tests, and `nova repl` provides interactive exploration. The type checker, tree-walking interpreter, arena memory system, and module manager are all operational. Active work is on LLVM codegen for `nova build`.

See the [Roadmap](ROADMAP.md) for detailed progress on every feature.

---

## Contributing

Nova is open source under the MIT license. Contributions are welcome — whether it's language design discussions, compiler work in Rust, documentation, or testing.

---

<p align="center">
  <strong>Nova</strong> — because writing fast software shouldn't be painful.
</p>

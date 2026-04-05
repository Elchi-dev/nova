```
 ███╗   ██╗ ██████╗ ██╗   ██╗ █████╗
 ████╗  ██║██╔═══██╗██║   ██║██╔══██╗
 ██╔██╗ ██║██║   ██║██║   ██║███████║
 ██║╚██╗██║██║   ██║╚██╗ ██╔╝██╔══██║
 ██║ ╚████║╚██████╔╝ ╚████╔╝ ██║  ██║
 ╚═╝  ╚═══╝ ╚═════╝   ╚═══╝  ╚═╝  ╚═╝
```

**A fast, ergonomic systems language with first-class distributed primitives.**

[![CI](https://github.com/Elchi-dev/nova/actions/workflows/ci.yml/badge.svg)](https://github.com/Elchi-dev/nova/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/Elchi-dev/nova?include_prereleases)](https://github.com/Elchi-dev/nova/releases)

---

Nova is a compiled, statically typed programming language that feels like Python but runs at native speed. It is built for developers who want ergonomic syntax without sacrificing performance, and who work in distributed environments where running code across machines should be as natural as calling a local function.

```python
# hello.nv

@runtime:
    memory: arena
    reload: hot

fn greet(name: str) -> str:
    return f"Hello, {name}!"

fn main():
    let msg = greet("Nova")
    print(msg)
```

```
$ nova run hello.nv
Hello, Nova!
```

---

## Why Nova?

Most languages make you choose between **ergonomics** and **performance**, or between **local** and **distributed** execution. Nova does not.

| | Nova | Python | Rust | Go |
|---|:---:|:---:|:---:|:---:|
| Native speed | ✅ | ❌ | ✅ | ✅ |
| Python-like syntax | ✅ | ✅ | ❌ | ❌ |
| No GC pauses | ✅ | ❌ | ✅ | ❌ |
| No borrow checker | ✅ | ✅ | ❌ | ✅ |
| First-class `dist` | ✅ | ❌ | ❌ | ❌ |
| Module hot-reload | ✅ | ❌ | ❌ | ❌ |

---

## Features

### Context-Aware Memory
No garbage collector. No borrow checker stress. Nova uses **arena-based memory** with compiler-driven escape analysis — objects are freed instantly when their scope ends, in bulk, with zero runtime overhead.

```python
fn process(data: []f64) -> f64:
    let result = sum(data)   # arena freed automatically when fn returns
    return result
```

### Module Hot-Reload
Change a function while your program runs. Nova compiles each module to an independent native unit. When you update a module, only that module is recompiled and swapped in — no restart, no lost state.

```python
module PhysicsEngine:
    @persist
    var gravity: f64 = 9.81   # survives a reload

    fn tick(dt: f64) -> f64:   # swap this live while simulation runs
        return gravity * dt
```

### First-Class Distributed Execution
Mark a function `@dist` and Nova handles serialization, dispatch, and error recovery across your cluster — at the language level, not the library level.

```python
@dist(node="worker-1")
fn compress(data: []Bytes) -> []Bytes:
    return heavy_compression(data)

fn main():
    let results = parallel map(compress, chunks)
```

### C Interoperability
Import C headers directly. No binding generators, no FFI boilerplate.

```python
import "libc.h"

fn main():
    let n = libc.printf("Hello from C\n")
```

### First-Class Decorators
```python
@runtime:
    memory: arena      # arena | gc
    reload: hot        # hot | cold
    dist: enabled      # enabled | disabled

@persist              # keep state across module reloads
@dist(node="worker")  # run on a specific cluster node
@checkpoint           # safe hot-reload point inside a loop
```

### Built-in Toolchain
Nova ships a single binary with everything you need.

```
nova build hello.nv        Compile to native binary
nova run   hello.nv        Compile and run
nova check hello.nv        Type-check without output
nova fmt                   Format all .nv files
nova lint                  Lint all .nv files
nova pkg add <name>        Add a package
nova new  <name>           Scaffold a new project
```

---

## Language Syntax

Nova uses significant indentation (like Python) with explicit `let`/`var` for immutability and optional type annotations.

```python
# Variables
let name = "Nova"         # immutable, type inferred
var count: i64 = 0        # mutable, explicit type

# Functions
fn add(a: i64, b: i64) -> i64:
    return a + b

# Control flow
fn classify(n: i64) -> str:
    if n < 0:
        return "negative"
    elif n == 0:
        return "zero"
    else:
        return "positive"

# For loops
fn sum_list(xs: []i64) -> i64:
    var total: i64 = 0
    for x in xs:
        total += x
    return total

# Structs
struct Point:
    x: f64
    y: f64

fn distance(a: Point, b: Point) -> f64:
    let dx = a.x - b.x
    let dy = a.y - b.y
    return (dx * dx + dy * dy)

# Modules
module Counter:
    @persist
    var count: i64 = 0

    fn increment():
        count += 1

    fn get() -> i64:
        return count
```

---

## Installation

> Nova is in early development. The compiler is not yet complete.
> Follow the repo for updates.

### Build from source

Requires Rust 1.75+.

```bash
git clone https://github.com/Elchi-dev/nova.git
cd nova
cargo build --release -p nova-cli
sudo cp target/release/nova /usr/local/bin/nova
```

---

## Project Structure

```
crates/
  nova-lexer/      Tokenizer — logos-based, full Nova token set
  nova-parser/     Parser + AST — chumsky-based, Python-like grammar
  nova-compiler/   Compilation pipeline (lex → parse → typecheck → codegen)
  nova-cli/        The `nova` CLI binary (build, run, check, fmt, lint, pkg)
examples/          Nova source examples
stdlib/            Nova standard library (planned)
docs/              Language reference (planned)
```

---

## Roadmap

| Feature | Status |
|---|---|
| Lexer — full token set | ✅ v0.1.0 |
| AST definition | ✅ v0.1.0 |
| CLI skeleton (build, run, check, fmt, lint, pkg) | ✅ v0.1.0 |
| CI — Ubuntu / macOS / Windows matrix | ✅ v0.1.0 |
| Parser — expressions + statements | 📅 v0.2.0 |
| Parser — modules + decorators | 📅 v0.2.0 |
| Type checker | 📅 v0.3.0 |
| LLVM codegen via inkwell | 📅 v0.4.0 |
| Arena memory + escape analysis | 📅 v0.5.0 |
| C FFI via libclang | 📅 v0.5.0 |
| Module hot-reload runtime | 📅 v0.6.0 |
| `@dist` distributed execution | 📅 v0.7.0 |
| Standard library | 📅 v0.8.0 |
| LSP server | 📅 v0.9.0 |
| Package manager | 📅 v1.0.0 |

---

## Contributing

Nova is in early development and contributions are very welcome.

```bash
git clone https://github.com/Elchi-dev/nova.git
cd nova
cargo check    # verify everything builds
cargo test     # run the test suite
```

Please run `cargo fmt --all` and `cargo clippy --all-targets --all-features -- -D warnings` before submitting a PR.

---

## License

MIT — [LICENSE](LICENSE) · built by [Elchi-dev](https://github.com/Elchi-dev)

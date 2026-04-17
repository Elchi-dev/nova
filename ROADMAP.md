# Nova Roadmap

> Last updated: April 2026
> Status: **Active Development — Pre-Alpha**

This roadmap tracks every feature of the Nova programming language — what's done, what's in progress, and what's planned. It also documents the hard technical challenges ("dragons") we're aware of and how we plan to solve them.

---

## Current Status: v0.1.0-dev

The full developer workflow is now in place: `nova init` scaffolds projects, `nova run` executes them, `nova check` type-checks, `nova fmt` formats, `nova test` runs tests, and `nova repl` provides interactive exploration. Under the hood: working lexer with line continuation, parser, AST, type checker with Hindley-Milner inference, tree-walking interpreter, arena memory allocator, and module hot-reload manager. Active work is on LLVM codegen for `nova build`.

---

## Phase 1 — Core Language (v0.1.0)

*Goal: Parse and type-check valid Nova programs. No code execution yet.*

| Feature | Status | Notes |
|---------|--------|-------|
| Lexer with indentation tracking | ✅ Done | Logos-based, emits Indent/Dedent tokens |
| Line continuation | ✅ Done | Multi-line expressions in brackets and after operators (`\|>`, `+`, etc.) |
| Token set (all operators, keywords) | ✅ Done | Including `\|>`, `=>`, `@`, effect brackets |
| Recursive descent parser | ✅ Done | Handles functions, structs, enums, traits, impl blocks |
| AST definitions | ✅ Done | Full node types for all language constructs |
| Tab-based + brace-based blocks | ✅ Done | `{}` with `;` for one-liners |
| Pipe operator parsing | ✅ Done | `data \|> transform \|> output` |
| Decorator parsing | ✅ Done | `@name` and `@name(args)` |
| Lambda expressions | ✅ Done | `x => x * 2` |
| Pattern matching (parsing) | ⚠️ Partial | `match/case` parsed, arm patterns need work |
| F-string parsing | 🔲 Todo | `f"hello {name}"` — lexer token exists, parser needs interpolation |
| Struct init expressions | ✅ Done | `Point { x: 1, y: 2 }` parsed in expression context |
| Type inference engine | ✅ Done | Hindley-Milner unification with substitution and occurs check |
| Type checker | ✅ Done | Validates types, mutability, scoping, operators, struct fields |
| Type environment with scoping | ✅ Done | Nested scopes, variable/function/type lookups |
| Built-in types & functions | ✅ Done | int, float, bool, str, list, dict + print, len, range, filter, map, sort |
| Error reporting | ⚠️ Partial | 22 error types defined, source location tracking in progress |

---

## Phase 2 — Semantic Analysis (v0.2.0)

*Goal: The compiler understands meaning, not just syntax.*

| Feature | Status | Notes |
|---------|--------|-------|
| Name resolution | ✅ Done | Scope tracking, variable binding, function lookups |
| Escape analysis | 🔲 Todo | Determine which values leave their scope |
| Effect inference | 🔲 Todo | Track `[io]`, `[error]` propagation through call graphs |
| Effect validation | ⚠️ Partial | `pure` functions cannot call effectful functions (basic check) |
| Contract verification | 🔲 Todo | Static checking of `require`/`ensure` where provable |
| Auto-module boundary detection | 🔲 Todo | Dependency graph analysis for hot-reload splitting |
| Semantic optimization passes | 🔲 Todo | Pattern fusion, redundancy elimination |
| Dead code detection | 🔲 Todo | Warn on unused variables, unreachable code |

---

## Phase 3 — Code Generation (v0.3.0)

*Goal: Nova programs compile and run.*

| Feature | Status | Notes |
|---------|--------|-------|
| Tree-walking interpreter (`nova run`) | ✅ Done | Full execution: functions, recursion, loops, pipes, structs, lambdas |
| Built-in functions | ✅ Done | print, len, range, str, abs, min, max, sum, sort, reverse, filter, map |
| String methods | ✅ Done | upper, lower, trim, contains, starts_with, ends_with, split, replace |
| Pipe operator execution | ✅ Done | `data \|> filter(pred) \|> map(fn) \|> sort` |
| Struct construction & field access | ✅ Done | `Point { x: 1, y: 2 }`, `p.x` |
| Lambda execution | ✅ Done | `x => x * 2` with closure capture |
| LLVM IR generation | 🔲 Todo | Using `inkwell` (LLVM Rust bindings) |
| Basic function compilation | 🔲 Todo | Functions → LLVM IR → machine code |
| Arena memory integration | ⚠️ Partial | Arena allocator implemented, codegen integration pending |
| Ref-count for escaped values | 🔲 Todo | Lightweight atomic refcount for ~5% of allocations |
| Struct layout / field access | 🔲 Todo | Memory layout calculation, field offset codegen |
| Enum / variant dispatch | 🔲 Todo | Tagged unions in LLVM IR |
| Trait vtable generation | 🔲 Todo | Dynamic dispatch via vtable pointers |
| Binary output (`nova build`) | 🔲 Todo | Linking, output binary |

---

## Phase 4 — Runtime & Hot-Reload (v0.4.0)

*Goal: The module system works end-to-end with live reloading.*

| Feature | Status | Notes |
|---------|--------|-------|
| Module manager | ✅ Done | Blue-green swap with state lifecycle |
| Module state machine | ✅ Done | Loading → Ready → Running → Idle → Draining → Retired |
| File watcher integration | 🔲 Todo | Detect source changes, trigger recompilation |
| Dynamic library loading | 🔲 Todo | `dlopen`/`dlsym` for swapping compiled modules |
| Message buffering during swap | 🔲 Todo | Queue calls while module is being replaced |
| Cross-module call routing | 🔲 Todo | Indirect calls through module dispatcher |
| Struct layout change detection | 🔲 Todo | Reject hot-reload if data layout changed (see Dragon #2) |
| Inlining boundary enforcement | 🔲 Todo | Prevent LLVM inlining across module boundaries (see Dragon #1) |
| State migration hooks | 🔲 Todo | Optional `@on_reload` decorator for state transformation |

---

## Phase 5 — FFI & Interop (v0.5.0)

*Goal: Import C libraries with a single line.*

| Feature | Status | Notes |
|---------|--------|-------|
| C header parsing | 🔲 Todo | Using `bindgen` or custom parser |
| Type mapping (C → Nova) | 🔲 Todo | `int` → `i32`, `char*` → `str`, etc. |
| Automatic binding generation | 🔲 Todo | Generate Nova functions from C signatures |
| Linking C libraries | 🔲 Todo | Static and dynamic linking support |
| Safety wrappers | 🔲 Todo | Wrap raw pointers in safe Nova types |
| C++ interop (future) | 🔲 Planned | After C is stable |
| Rust crate interop (future) | 🔲 Planned | Long-term goal |

---

## Phase 6 — CLI & Tooling (v0.6.0)

*Goal: A complete, polished developer experience from day one.*

| Feature | Status | Notes |
|---------|--------|-------|
| `nova run` | ✅ Done | Lex → parse → type-check → execute (tree-walking interpreter) |
| `nova build` | 🔲 Stub | Needs codegen (Phase 3) |
| `nova check` | ✅ Done | Full pipeline: lex → parse → type-check with error reporting |
| `nova fmt` | ✅ Done | AST-based formatter, walks directories, `--check` mode |
| `nova test` | ✅ Done | Auto-discovers `test_*` functions, runs with timing, filter support |
| `nova doc` | 🔲 Stub | Generate HTML docs from doc comments |
| `nova repl` | ✅ Done | Interactive REPL with multi-line blocks, `:help`, `:clear`, persistent env |
| `nova init` | ✅ Done | Project scaffolding: `nova.toml`, `src/`, `tests/`, README, `.gitignore` |
| `nova mod add/remove/update` | 🔲 Stub | Package registry and dependency resolution |
| LSP server | 🔲 Planned | Editor support for VS Code, Neovim, etc. |
| `nova build --explain` | 🔲 Planned | Show semantic optimizations applied |

---

## Phase 7 — Concurrency (v0.7.0)

*Goal: Structured concurrency integrated with the arena memory model.*

| Feature | Status | Notes |
|---------|--------|-------|
| `spawn` keyword | 🔲 Todo | Create child tasks bound to parent scope |
| `await` keyword | 🔲 Todo | Wait for task completion |
| Scope-based task lifetime | 🔲 Todo | Tasks cannot outlive their parent scope |
| Per-task arenas | 🔲 Todo | Each spawned task gets its own memory arena |
| Channel-based communication | 🔲 Todo | Type-safe message passing between tasks |
| Work-stealing scheduler | 🔲 Todo | Efficient multi-core task distribution |

---

## Phase 8 — Standard Library (v0.8.0)

*Goal: A useful standard library for real-world programs.*

| Feature | Status | Notes |
|---------|--------|-------|
| `std.io` | 🔲 Todo | File I/O, stdin/stdout, stderr |
| `std.fs` | 🔲 Todo | Filesystem operations: read, write, mkdir, walk, glob |
| `std.net` | 🔲 Todo | TCP/UDP sockets, HTTP client |
| `std.http` | 🔲 Todo | HTTP server, request/response, routing |
| `std.json` | 🔲 Todo | JSON parsing and serialization |
| `std.csv` | 🔲 Todo | CSV reading and writing |
| `std.collections` | 🔲 Todo | HashMap, Set, Queue, Stack, Deque |
| `std.math` | 🔲 Todo | Math functions, constants, trigonometry |
| `std.random` | 🔲 Todo | Random number generation, shuffling |
| `std.time` | 🔲 Todo | Timestamps, durations, formatting, sleep |
| `std.fmt` | 🔲 Todo | String formatting, f-string runtime |
| `std.regex` | 🔲 Todo | Regular expressions |
| `std.os` | 🔲 Todo | Environment variables, process spawning, signals |
| `std.path` | 🔲 Todo | Cross-platform path manipulation |
| `std.test` | 🔲 Todo | Test framework, assertions, benchmarks |
| `std.log` | 🔲 Todo | Structured logging with levels |
| `std.crypto` | 🔲 Planned | Hashing (SHA, Blake3), HMAC, encryption |
| `std.encoding` | 🔲 Planned | Base64, hex, URL encoding |
| `std.args` | 🔲 Planned | CLI argument parsing |

---

## Phase 9 — Ecosystem & Editor Support (v0.9.0)

*Goal: First-class developer experience in popular editors and a growing ecosystem.*

| Feature | Status | Notes |
|---------|--------|-------|
| VSCode extension | 🔲 Planned | Syntax highlighting, bracket matching, file icons |
| VSCode LSP integration | 🔲 Planned | Autocomplete, go-to-definition, hover types, inline errors |
| Treesitter grammar | 🔲 Planned | Syntax highlighting for Neovim, Helix, Zed, etc. |
| Syntax theme colors | 🔲 Planned | Custom token colors tuned for Nova keywords and operators |
| `.nova` and `.nv` file association | 🔲 Planned | Both extensions recognized across all tooling |
| Package registry (`nova.pkg`) | 🔲 Planned | Central package registry for community libraries |
| `nova mod publish` | 🔲 Planned | Publish packages to the registry |
| Playground (web) | 🔲 Planned | Try Nova in the browser (WASM-based interpreter) |
| Starter templates | 🔲 Planned | `nova init --template web/cli/lib` scaffolding |
| CI/CD integration | 🔲 Planned | GitHub Actions for Nova projects |

---

## Known Technical Challenges

These are hard problems identified during design. Each one has a planned approach.

### Dragon #1: The Inlining Paradox

**Problem:** LLVM's main speed trick is inlining — copying function code into the caller. But if Function A (Module 1) inlines Function B (Module 2), and you hot-reload Module 2, Function A still has the old code baked in.

**Our approach:**
- Modules are compiled as separate LLVM compilation units
- Cross-module calls use indirect call dispatch (function pointers through the module table)
- LLVM cannot inline across module boundaries by design
- Intra-module inlining remains fully enabled
- `nova build --release` (no hot-reload) enables full cross-module inlining for maximum performance
- Two compilation modes: **dev** (hot-reloadable, slight call overhead) and **release** (fully optimized, no hot-reload)

**Performance cost:** ~2-5ns per cross-module call in dev mode (indirect vs direct call). Negligible for application code, and eliminated entirely in release builds.

### Dragon #2: State Migration on Struct Changes

**Problem:** If you add a field to `struct Player` while the program is running, existing `Player` instances in memory have the wrong layout. In native code, memory layouts are baked into the binary as fixed offsets.

**Our approach:**
- The compiler tracks struct layouts per module version
- On hot-reload, if a struct's layout changed, the reload is **rejected with a clear error**: `"cannot hot-reload: Player layout changed (added field 'level'). Restart required."`
- This is the safe default — behavior changes are reloadable, data shape changes are not
- Future: optional `@on_reload` decorator for explicit state migration (like Erlang's `code_change`), but never automatic or silent
- `nova build --explain` will show exactly why a reload was rejected

### Dragon #3: The Escape Analysis Performance Cliff

**Problem:** A tiny code change (returning a value instead of processing it locally) can flip an allocation from arena (fast) to ref-counted (slower). The developer might not notice.

**Our approach:**
- `nova check` will include **allocation reports**: "This function has 3 arena allocations and 1 escaped allocation"
- Warnings when escape count increases between edits: `"warning: 'items' now escapes this scope (was arena-local). Consider processing it before returning."`
- The `@arena` decorator can force arena-only allocation (compile error if anything escapes)
- `nova build --profile-alloc` generates a full allocation heatmap
- The performance difference is still small (ref-count vs arena), not catastrophic. This is about awareness, not emergencies.

### Dragon #4: Compilation Speed

**Problem:** LLVM is slow to compile. Rust's compile times are the #1 complaint. If Nova uses LLVM, won't it be slow too?

**Our approach:**
- `nova run` uses a **tree-walking interpreter** — no LLVM, instant feedback
- `nova build` in debug mode uses **Cranelift** (faster codegen, less optimization) instead of LLVM
- `nova build --release` uses LLVM for maximum optimization
- Hot-reloading only recompiles the changed module, not the entire program
- Incremental compilation caches between builds

---

## Release Timeline (Estimated)

| Version | Target | Milestone |
|---------|--------|-----------|
| v0.1.0 | ~~Q2 2026~~ ✅ | Parser + type checker complete, `nova check` works |
| v0.1.1 | ~~Q2 2026~~ ✅ | Tree-walking interpreter, `nova run` executes programs |
| v0.2.0 | Q3 2026 | Semantic analysis, escape analysis, effect system |
| v0.3.0 | Q4 2026 | LLVM code generation, `nova build` produces binaries |
| v0.4.0 | Q1 2027 | Hot-reloading works end-to-end |
| v0.5.0 | Q2 2027 | C interop functional |
| v0.6.0 | Q3 2027 | CLI tools complete, LSP server |
| v0.7.0 | Q4 2027 | Structured concurrency |
| v0.8.0 | Q1 2028 | Standard library |
| v0.9.0 | Q2 2028 | VSCode extension, ecosystem tooling, package registry |
| v1.0.0 | 2028 | Stable release |

---

## How to Contribute

Nova is open source (MIT). We welcome contributions at every level:

- **Language design:** Open an issue to discuss syntax, semantics, or new features
- **Compiler work:** The lexer, parser, and runtime are in Rust — PRs welcome
- **Documentation:** Help explain Nova to the world
- **Testing:** Write Nova programs and report what breaks

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

*This roadmap is a living document. It will be updated as development progresses.*

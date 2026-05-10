# Nova Roadmap

> Last updated: May 2026
> Status: **Active Development — Pre-Alpha**

This roadmap tracks every feature of the Nova programming language — what's done, what's in progress, and what's planned. It also documents the hard technical challenges ("dragons") we're aware of and how we plan to solve them.

---

## Current Status: v0.1.4-dev

The full developer workflow is in place and the CI is green across Ubuntu, macOS, and Windows. Nova can parse and execute real programs: the parser handles the full language surface (structs, enums with generics, traits, impl blocks, pattern matching, f-strings, contracts, effects, decorators). The tree-walking interpreter runs Nova programs end-to-end. Active work is now on **LLVM codegen** — making `nova build` produce real native binaries.

---

## Phase 1 — Core Language (v0.1.x) ✅

*Goal: Parse and type-check valid Nova programs. Execute them via tree-walking interpreter.*

| Feature | Status | Notes |
|---------|--------|-------|
| Lexer with indentation tracking | ✅ Done | Logos-based, emits Indent/Dedent tokens |
| Line continuation | ✅ Done | Multi-line expressions in brackets and after operators (`\|>`, `+`, etc.) |
| Token set (all operators, keywords) | ✅ Done | Including `\|>`, `=>`, `@`, `pure`, `require`, `ensure`, effect brackets |
| Recursive descent parser | ✅ Done | Functions, structs, enums, traits, impl blocks, match, f-strings, contracts |
| AST definitions | ✅ Done | Full node types for all language constructs |
| Tab-based + brace-based blocks | ✅ Done | `{}` with `;` for one-liners |
| Pipe operator parsing + execution | ✅ Done | `data \|> filter(x => x > 0) \|> map(x => x * 2) \|> sort` |
| Decorator parsing | ✅ Done | `@name` and `@name(args)` |
| Lambda expressions | ✅ Done | `x => x * 2` with closure capture |
| F-string parsing | ✅ Done | `f"hello {name}"` — full interpolation with nested expressions |
| Pattern matching | ✅ Done | `match/case` with wildcards, literals, variant patterns |
| Enum generics | ✅ Done | `enum Result[T, E]:` with typed variant payloads |
| Trait method signatures | ✅ Done | Bodyless signatures in trait blocks |
| `self` in methods | ✅ Done | `fn draw(self) -> none:` — self as param and expression |
| `require` / `ensure` statements | ✅ Done | Design by contract — parsed, AST nodes, runtime assertions |
| `pure fn` modifier | ✅ Done | Marks functions as side-effect-free |
| Foreign import syntax | ✅ Done | `import foreign("header.h", lang: "c")` fully parsed |
| Struct fields without defaults | ✅ Done | `let name: str` — value optional in struct field declarations |
| Struct init expressions | ✅ Done | `Point { x: 1, y: 2 }` in expression context |
| Type inference engine | ✅ Done | Hindley-Milner unification with substitution and occurs check |
| Type checker | ✅ Done | Validates types, mutability, scoping, operators, struct fields |
| Built-in types & functions | ✅ Done | int, float, bool, str, list, dict + print, len, range, filter, map, sort |
| Tree-walking interpreter (`nova run`) | ✅ Done | Full execution: functions, recursion, loops, pipes, structs, lambdas |
| Arena allocator | ✅ Done | Bump allocator with O(1) bulk free — not yet wired to codegen |
| Module hot-reload manager | ✅ Done | Blue-green swap with lifecycle state machine — not yet wired to runtime |
| `nova run --watch` | ✅ Done | Auto-reruns on file changes (notify crate, 100ms debounce) |
| `nova check` | ✅ Done | Full pipeline: lex → parse → type-check with error reporting |
| `nova fmt` | ✅ Done | AST-based formatter, walks directories, `--check` mode |
| `nova test` | ✅ Done | Auto-discovers `test_*` functions, runs with timing, filter support |
| `nova doc` | ✅ Done | HTML docs from `##` doc comments with syntax-highlighted theme |
| `nova repl` | ✅ Done | Interactive REPL with multi-line blocks, `:help`, `:clear`, persistent env |
| `nova init` | ✅ Done | Project scaffolding: `nova.toml`, `src/`, `tests/`, README, `.gitignore` |
| CI (GitHub Actions) | ✅ Done | Tests across ubuntu/macos/windows, clippy, rustfmt, example smoke tests |
| Error reporting | ⚠️ Partial | 22 error types defined, source locations tracked — rich display pending |

---

## Phase 2 — Native Compilation (v0.2.0) ← **Active**

*Goal: `nova build hello.nova` produces a real native binary that runs at LLVM-optimized speed. Arena memory genuinely active. Basic standard library.*

| Feature | Status | Notes |
|---------|--------|-------|
| `inkwell` LLVM bindings | 🔲 Todo | Add to workspace, pin LLVM 18, document install in CONTRIBUTING.md |
| LLVM codegen skeleton | 🔲 Todo | `Codegen` struct wrapping LLVM context + module + builder |
| Integer / float literals | 🔲 Todo | Constants → LLVM IR |
| Arithmetic & comparisons | 🔲 Todo | add, sub, mul, sdiv, icmp, fcmp → LLVM IR |
| Variable bindings (`let`) | 🔲 Todo | `alloca` + `store` + `load` |
| If / else | 🔲 Todo | LLVM `br` + basic blocks |
| While / for loops | 🔲 Todo | Backedge + loop exit branch |
| Function definitions + calls | 🔲 Todo | LLVM function emission, `call` instruction |
| Return values | 🔲 Todo | `ret` instruction |
| Escape analysis pass | 🔲 Todo | Conservative: returned values escape, rest go to arena |
| Arena integration in codegen | 🔲 Todo | Function entry creates arena, exit frees it; heap for escaped values |
| Struct layout + field access | 🔲 Todo | Memory layout, `getelementptr` for field access |
| String / list representation | 🔲 Todo | `{ ptr, len, capacity }` structs in LLVM IR |
| Emit object file + link binary | 🔲 Todo | LLVM passes → `.o` → invoke system linker |
| `nova build` command | 🔲 Todo | Wire codegen into CLI, produce executable |
| `std.io` (minimum) | 🔲 Todo | `print`, `println`, `read_line`, `read_file`, `write_file` |
| `std.math` (minimum) | 🔲 Todo | Constants + `sqrt`, `pow`, trig, `floor`, `ceil`, `abs` |
| `std.list` (minimum) | 🔲 Todo | `map`, `filter`, `reduce`, `sort`, `reverse`, `range`, `len` |
| `std.string` (minimum) | 🔲 Todo | `parse_int`, `parse_float` + existing methods |
| Module import resolution | 🔲 Todo | `import std.io` → look up bundled stdlib file, bring symbols into scope |
| Enums (tagged unions) | 🔲 Todo | Discriminator + max-sized payload in LLVM IR |
| Pattern matching codegen | 🔲 Todo | Compile `match/case` to discriminator checks + payload extraction |
| Generics (monomorphization) | 🔲 Todo | Each instantiation → separate concrete LLVM function |
| F-string codegen | 🔲 Todo | Interpolate values using type-specific formatters, concatenate |
| `nova check --profile-alloc` | 🔲 Todo | Per-function allocation breakdown: arena vs escaped vs static |
| Fibonacci benchmark | 🔲 Todo | Compare native binary vs Python vs C — must be within 2x of C |

---

## Phase 3 — Runtime & Hot-Reload (v0.3.0)

*Goal: The hot-reload system works end-to-end. Change a file, the running program updates.*

| Feature | Status | Notes |
|---------|--------|-------|
| Module manager | ✅ Done | Blue-green swap with lifecycle state machine |
| Module state machine | ✅ Done | Loading → Ready → Running → Idle → Draining → Retired |
| Auto-module boundary detection | 🔲 Todo | Dependency graph → split into hot-reloadable modules |
| File watcher integration | 🔲 Todo | Detect source changes, trigger recompilation of changed module |
| Dynamic library loading | 🔲 Todo | `dlopen`/`dlsym` for swapping compiled modules at runtime |
| Cross-module call routing | 🔲 Todo | Indirect calls through module dispatch table (dev mode) |
| Struct layout change detection | 🔲 Todo | Reject hot-reload if data layout changed (Dragon #2) |
| Inlining boundary enforcement | 🔲 Todo | Prevent LLVM inlining across module boundaries (Dragon #1) |
| `@on_reload` decorator | 🔲 Planned | Explicit state migration hook, like Erlang's `code_change` |

---

## Phase 4 — FFI & Interop (v0.4.0)

*Goal: Import any C library with one line.*

| Feature | Status | Notes |
|---------|--------|-------|
| C header parsing | 🔲 Todo | Using `bindgen` to parse C headers |
| Type mapping (C → Nova) | 🔲 Todo | `int` → `i32`, `char*` → `str`, pointers → safe wrappers |
| Automatic binding generation | 🔲 Todo | Generate Nova function signatures from C declarations |
| Linking C libraries | 🔲 Todo | Static and dynamic linking support |
| C++ interop | 🔲 Planned | After C is stable |

---

## Phase 5 — Language Completeness (v0.5.0)

*Goal: The language is complete enough for real-world programs.*

| Feature | Status | Notes |
|---------|--------|-------|
| Trait system with dynamic dispatch | 🔲 Todo | vtable generation in LLVM IR |
| Effect inference (full) | 🔲 Todo | Track `[io]`, `[error]` propagation through entire call graphs |
| Contract verification (static) | 🔲 Todo | Prove `require`/`ensure` at compile time where possible |
| Async / await | 🔲 Todo | Structured concurrency with scope-bound tasks |
| Optional parameters + keyword args | 🔲 Todo | `fn greet(name: str, greeting: str = "Hello")` |
| `?` operator | 🔲 Todo | Early-return sugar for Result types |
| Tuple types | 🔲 Todo | `(int, str)` with destructuring |
| Slice syntax | 🔲 Todo | `list[1:5]`, `list[2:]`, `list[:3]` |
| List comprehensions | 🔲 Planned | `[x * 2 for x in items if x > 0]` |
| Rich error messages | 🔲 Todo | Source snippets with arrows via `miette` |

---

## Phase 6 — Standard Library (v0.6.0)

*Goal: A useful stdlib for real-world programs.*

| Feature | Status | Notes |
|---------|--------|-------|
| `std.io` | 🔲 Todo | File I/O, stdin/stdout, stderr |
| `std.fs` | 🔲 Todo | Filesystem: read, write, mkdir, walk, glob |
| `std.net` | 🔲 Todo | TCP/UDP sockets, HTTP client |
| `std.json` | 🔲 Todo | JSON parsing and serialization |
| `std.collections` | 🔲 Todo | HashMap, Set, Queue, Stack |
| `std.math` | 🔲 Todo | Math functions and constants |
| `std.time` | 🔲 Todo | Timestamps, durations, formatting |
| `std.os` | 🔲 Todo | Env vars, process spawning, signals |

---

## Phase 7 — Ecosystem (v0.7.0+)

| Feature | Status | Notes |
|---------|--------|-------|
| VSCode extension | 🔲 Planned | Syntax highlighting, file icons |
| LSP server | 🔲 Planned | Autocomplete, go-to-definition, hover types |
| Treesitter grammar | 🔲 Planned | Neovim, Helix, Zed support |
| Package registry | 🔲 Planned | `nova mod publish`, community packages |
| Web playground | 🔲 Planned | Try Nova in the browser (WASM) |

---

## Known Technical Challenges

### Dragon #1: The Inlining Paradox

**Problem:** LLVM's main speed trick is inlining. But if Function A inlines Function B across a module boundary, hot-reloading Module B doesn't update Function A — it still has the old code baked in.

**Our approach:** Two compilation modes. **Dev mode** compiles each module as a separate LLVM compilation unit with cross-module calls going through a dispatch table (indirect calls, ~2-5ns overhead). LLVM cannot inline across compilation unit boundaries. **Release mode** (`nova build --release`) enables full inlining for maximum performance — no hot-reload, but C-comparable speed. The developer chooses: live reloading in dev, maximum performance in release.

---

### Dragon #2: Struct Layout Changes

**Problem:** Adding a field to `struct Player` while the program runs would corrupt existing `Player` instances — native code has field offsets baked in as constants.

**Our approach:** The compiler tracks struct layouts per module version. If a struct's layout changed, hot-reload is **rejected with a clear error**: `"cannot hot-reload: Player layout changed (added field 'level'). Restart required."` Behavior changes reload; shape changes require restart. Future: optional `@on_reload` for explicit state migration, never automatic.

---

### Dragon #3: The Escape Analysis Performance Cliff

**Problem:** A small code change — returning a value instead of processing it locally — can silently flip an allocation from arena (2 instructions) to ref-counted (slower). The developer might not notice.

**Our approach:** `nova check --profile-alloc` reports per-function allocation breakdowns. Warnings when escape behavior changes between edits. The `@arena` decorator forces arena-only allocation (compile error if anything escapes). The performance difference is real but not catastrophic — the goal is awareness.

---

## Release Timeline

| Version | Status | Milestone |
|---------|--------|-----------|
| v0.1.0-dev | ✅ Released | Parser + type checker, `nova check` works |
| v0.1.1-dev | ✅ Released | Tree-walking interpreter, `nova run` executes programs |
| v0.1.2-dev | ✅ Released | Full CLI tooling — fmt, test, repl, init |
| v0.1.3-dev | ✅ Released | Doc generator, `--watch`, GitHub Actions CI |
| v0.1.4-dev | ✅ Released | Parser completeness — contracts, enums, traits, f-strings, match/case. CI green. |
| **v0.2.0** | 🔲 **Active** | **LLVM native compilation — `nova build` produces real binaries. Arena memory active. Basic stdlib.** |
| v0.3.0 | 🔲 Planned | Hot-reloading end-to-end |
| v0.4.0 | 🔲 Planned | C FFI — `import foreign("header.h")` works |
| v0.5.0 | 🔲 Planned | Language completeness — traits, effects, async |
| v0.6.0 | 🔲 Planned | Full standard library |
| v0.7.0+ | 🔲 Planned | Ecosystem — VSCode extension, LSP, package registry |
| v1.0.0 | 🔲 Planned | Stable release |

---

## How to Contribute

Nova is open source (MIT). Contributions welcome at every level:

- **Language design:** Open an issue to discuss syntax, semantics, or new features
- **Compiler work:** The lexer, parser, type checker, and runtime are in Rust — PRs welcome
- **Documentation:** Help explain Nova to the world
- **Testing:** Write Nova programs and report what breaks

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

*This roadmap is a living document. Updated with every release.*

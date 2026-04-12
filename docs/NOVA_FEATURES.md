# Nova — The Programming Language That Shouldn't Exist Yet

## What is Nova?

Nova is a new compiled programming language. It compiles to native machine code via LLVM, so it's as fast as C or Rust. But it reads like Python. And it has features that no other language has ever combined in one place.

The compiler is written in Rust. The language targets both systems programming and application development — the idea being that you shouldn't have to choose between "easy to write" and "fast to run."

Here's what a Nova program looks like:

```python
import std.io

pub struct Player:
    let name: str
    let mut health: int = 100

@cached
fn fibonacci(n: int) -> int:
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

fn main():
    let player = Player { name: "Nova", health: 100 }

    let result = [1, 2, 3, 4, 5]
        |> filter(x => x > 0)
        |> map(x => x * 2)

    io.print(f"Player: {player.name}")
    io.print(f"Fib(10) = {fibonacci(10)}")
```

If you know Python, you can already read this. But underneath, this compiles to a native binary that runs at C speed. No interpreter, no VM, no JIT warmup.

Now let's talk about what makes Nova different from everything else.

---

## 1. Context-Aware Memory

**The problem:** Every language makes you choose your poison for memory management.

- **C/C++:** Manual management. Fast, but you'll have use-after-free bugs, memory leaks, and segfaults. Entire classes of security vulnerabilities exist because of this.
- **Java/Go/Python:** Garbage collector. Easy to use, but GC pauses destroy latency. Your game stutters, your server hiccups, your real-time system misses deadlines.
- **Rust:** Borrow checker. No GC, no leaks — but the learning curve is brutal. You'll fight the compiler for weeks before it clicks. Many developers bounce off Rust entirely.

**Nova's solution:** Context-Aware Memory.

The core idea is simple: most objects in a program are born, used, and die within the same scope. They don't escape. A function creates some local variables, does some work, and returns. Those locals don't need individual tracking — they can all be freed together when the scope ends.

Nova uses **arena-based allocation**. When a scope starts (a function call, a loop body, a block), an arena is created — a big chunk of memory. Every allocation within that scope is a simple bump pointer into the arena. When the scope ends, the entire arena is freed in one operation. Not one-by-one. All at once. O(1).

But what about objects that *do* escape? A value returned from a function, stored in a global, or shared across threads? The compiler runs **escape analysis** at compile time. It statically determines which objects leave their scope. Only those — typically around 5% of all allocations — get promoted to lightweight reference counting.

The result:

- **~95% of allocations:** Arena bump allocation (2 CPU instructions) + bulk free (1 operation)
- **~5% of allocations:** Reference counted, automatically
- **0% GC pauses:** There is no garbage collector
- **0% borrow checker fights:** There is no borrow checker

You write code like you would in Python. The compiler handles the rest.

```python
fn process_data(items: list[int]) -> list[int]:
    # Everything in here lives in a scope arena.
    # 'filtered' and 'doubled' are arena-allocated.
    let filtered = items |> filter(x => x > 0)
    let doubled = filtered |> map(x => x * 2)

    # 'doubled' is returned — escape analysis detects this.
    # Only 'doubled' gets promoted to refcount.
    # 'filtered' is freed in bulk when this function returns.
    return doubled
```

---

## 2. Transparent Hot-Reloading via Auto-Modules

**The problem:** When you change code, you restart the program. For a web server, that means dropped connections. For a game, that means reloading the entire level. For a long-running data pipeline, that means starting over.

Some languages have tried to solve this. Erlang/Elixir has hot code reloading, but it requires you to think in OTP modules, write `code_change` callbacks, and carefully manage state migration. It's powerful but demands architectural commitment from day one.

**Nova's solution:** You write normal code. The compiler handles the rest.

Here's how it works:

1. **Compile-time module splitting:** The Nova compiler analyzes your code's dependency graph and automatically partitions it into modules at compilation boundaries. Functions that call each other tightly stay in one module. Independent subsystems become separate modules. You don't declare modules — the compiler infers them.

2. **Module lifecycle:** Every module has a state: `Loading`, `Ready`, `Running`, `Idle`, `Draining`, `Retired`. The runtime tracks which modules are actively handling calls.

3. **Blue-green swapping:** When you change a file and trigger a reload, the affected module is recompiled. The new version is staged alongside the old one. New calls go to the new version. The old version finishes its active calls (draining). Once drained, it's retired. No request is ever lost. No state is corrupted.

4. **Message buffering:** During the swap window, calls between modules are buffered in a message queue. This handles the edge case where a module is called while it's being replaced.

From the developer's perspective, nothing changes. You write code. You save the file. The change is live. The program never stops.

```python
# You write this normally. The compiler splits it into modules.
# Change this function, save, and it's live in your running server.

fn handle_request(req: Request) -> Response:
    let user = auth.verify(req.token)
    let data = db.query(user.id)
    return Response { status: 200, body: data }
```

No other compiled language does this automatically. Erlang requires manual module design. Nim requires manual `performCodeReload()` calls. Flutter only does it for UI in development. Nova does it transparently, for all code, in production.

---

## 3. Python-Like Syntax with Performance

**The syntax:** Tab-based indentation by default, just like Python. No semicolons, no curly braces cluttering your code.

But sometimes you want a quick one-liner. Nova supports an inline brace mode:

```python
# Normal indentation-based blocks
fn process(data: list[int]) -> int:
    let filtered = data |> filter(x => x > 0)
    return filtered |> sum

# One-liner with braces (semicolons required inside braces)
fn double(x: int) -> int { return x * 2; }
fn square(x: int) -> int { return x * x; }
```

Both styles compile to the same thing. Use whichever fits the moment.

**The type system** is statically typed with full type inference. You declare types when you want clarity, and the compiler infers them when it's obvious:

```python
let x = 42              # Compiler infers: int
let name = "Nova"       # Compiler infers: str
let items: list[int] = [1, 2, 3]  # Explicit when you want it
```

---

## 4. Direct C Interop

**The problem:** You want to use an existing C library — SDL, OpenGL, SQLite, whatever. In most languages, you need to write bindings: wrapper functions, type mappings, memory bridging. It's tedious and error-prone.

**Nova's solution:** Point at the header file.

```python
import foreign("raylib.h", lang: "c")
import foreign("sqlite3.h", lang: "c")
```

The compiler parses the C header, generates type-safe Nova bindings, and links the library. No wrapper code. No FFI ceremony. You call C functions as if they were Nova functions.

This is designed to be extensible — C first, then C++ and Rust crate support later.

---

## 5. Built-in Decorators (Compile-Time)

Python has decorators, but they're runtime function wrappers — they add overhead and are limited to what you can do at runtime.

Nova's decorators are **compile-time code transformations**. The `@cached` decorator doesn't wrap your function in another function at runtime. It rewrites your function at compile time to include a cache lookup. Zero runtime overhead from the decorator mechanism itself.

```python
@cached                    # Memoization — added at compile time
@log_time                  # Execution timing — injected at compile time
@validate                  # Input validation — checked at compile time where possible
fn expensive_computation(data: list[int]) -> int:
    return data |> map(x => x ** 2) |> sum

@retry(max_attempts: 3)    # Decorators can take arguments
fn fetch_data(url: str) -> str [io, error]:
    return http.get(url)
```

Because decorators are compile-time, the compiler can reason about them. It knows `@cached` means the function is pure (otherwise caching is wrong). It can verify that `@validate` constraints are satisfiable. It can optimize `@log_time` away in release builds.

---

## 6. Effect System

**The problem:** In most languages, any function can do anything. A function called `calculate_tax()` might read a file, make a network request, or launch missiles. You can't tell from the signature.

This makes testing hard (you need mocks for everything), reasoning hard (what does this function actually do?), and optimization hard (can the compiler reorder these calls?).

**Nova's solution:** Functions declare their side effects.

```python
# This function is pure — no side effects allowed.
# The compiler enforces this. If you try to do IO here, it won't compile.
pure fn add(a: int, b: int) -> int:
    return a + b

# This function does IO and might fail.
# The effects are part of the signature.
fn read_config(path: str) -> str [io, error]:
    return io.read_file(path)

# This function only does IO, it cannot fail.
fn log_message(msg: str) -> none [io]:
    io.print(msg)
```

The `pure` keyword and `[effect]` annotations aren't just documentation. The compiler tracks effect propagation: a function that calls an `[io]` function must itself declare `[io]`. A `pure` function cannot call anything with effects.

This means:
- **Pure functions are guaranteed safe to cache, parallelize, and reorder**
- **You can see at a glance what any function does to the outside world**
- **Testing pure functions requires zero mocks**

---

## 7. Design by Contract

Built-in preconditions and postconditions, not as a library but as language syntax:

```python
fn divide(a: float, b: float) -> float:
    require b != 0.0            # Precondition: caller's responsibility
    ensure result * b == a      # Postcondition: function's guarantee
    return a / b

fn withdraw(account: Account, amount: float) -> Account:
    require amount > 0.0
    require amount <= account.balance
    ensure result.balance == account.balance - amount
    return Account { balance: account.balance - amount }
```

The compiler verifies contracts statically where possible. If it can prove at compile time that `b` is never zero, it eliminates the runtime check entirely. If it can't prove it, the check runs in debug builds and can be stripped in release builds.

This catches bugs at the boundary between components — exactly where most real-world bugs live.

---

## 8. Pipe Operator

Borrowed from Elixir and F#. Instead of nested function calls that read inside-out, you chain operations left-to-right:

```python
# Without pipes — read inside-out, right-to-left
result = sort(filter(map(data, x => x * 2), x => x > 10))

# With pipes — read left-to-right, top-to-bottom
result = data
    |> map(x => x * 2)
    |> filter(x => x > 10)
    |> sort
```

This isn't just syntactic sugar. The pipe operator integrates with Nova's semantic-aware compilation (see below) — the compiler can fuse piped operations into a single pass.

---

## 9. Semantic-Aware Compilation

Most compilers optimize at the instruction level: register allocation, loop unrolling, dead code elimination. They understand *syntax* but not *meaning*.

Nova's compiler understands semantics — what your code is trying to do — and optimizes at that level:

**Pattern Fusion:** `sort |> reverse` is recognized as "sort descending" and compiled to a single reverse-sort pass instead of two separate operations. `map(f) |> map(g)` becomes `map(f∘g)` — one loop instead of two.

**Redundancy Elimination:** `filter(x > 0) |> filter(x > 5)` — the compiler recognizes that the second filter subsumes the first and eliminates it.

**Data Structure Selection:** If the compiler sees that you only do lookups on a collection (never iterate in order), it can internally use a hash map even though you wrote `list`. Only when ordering doesn't matter.

**The guarantee:** The compiler never changes the *result* of your code, only the *path* to get there. Same output, fewer operations. And if you want to opt out: `@literal` forces the compiler to do exactly what you wrote.

```python
# The compiler optimizes this automatically:
result = data
    |> map(x => x * 2)        # ┐
    |> map(x => x + 1)        # ┘ Fused into one map: x => x * 2 + 1
    |> filter(x => x > 0)     # ┐
    |> filter(x => x > 10)    # ┘ Reduced to one filter: x => x > 10
    |> sort
    |> reverse                 # Fused with sort into reverse-sort
```

You can always run `nova build --explain` to see exactly what optimizations were applied.

---

## 10. Structured Concurrency

No `async/await` spaghetti. No orphaned tasks. Nova uses scope-based concurrency where tasks are tied to their parent scope:

```python
fn fetch_all(urls: list[str]) -> list[Response]:
    # All spawned tasks are bound to this scope.
    # If this function returns or crashes, all tasks are cleaned up.
    # No task can outlive its parent. Ever.
    let tasks = urls |> map(url => spawn fetch(url))
    return tasks |> map(t => await t)
```

This integrates perfectly with the arena memory model — each concurrent scope gets its own arena, and cleanup is deterministic.

---

## 11. Capability-Based Security

Every module gets explicit capabilities. A module that only does math physically cannot access the file system — not because a runtime blocks it, but because the compiler doesn't generate the code for it.

```python
# This module has no [io] capability.
# It literally cannot read files or make network calls.
# The compiler won't compile any IO operations here.
pure fn process(data: list[int]) -> int:
    return data |> sum
```

This is security at the compiler level, not the runtime level. There's no permission system to bypass. The capability simply doesn't exist.

---

## The CLI

Nova ships as a single binary with everything built in:

| Command | What it does |
|---------|-------------|
| `nova run file.nova` | Compile and execute |
| `nova build` | Compile to native binary (LLVM) |
| `nova check` | Type-check + lint (no compilation) |
| `nova fmt` | Auto-format source files |
| `nova test` | Run test suite |
| `nova doc` | Generate documentation |
| `nova repl` | Interactive shell |
| `nova init name` | Scaffold a new project |
| `nova mod add pkg` | Dependency management |

No external tools needed. No `pip install formatter`. No `npm install linter`. Everything is day-one, built-in, consistent.

---

## How Nova Compares

| Feature | Python | Go | Rust | Nova |
|---------|--------|-----|------|------|
| Syntax readability | Excellent | Good | Complex | Excellent |
| Performance | Slow (interpreted) | Fast | Fastest | Fast (LLVM) |
| Memory management | GC | GC | Borrow checker | Arena + escape analysis |
| Learning curve | Low | Low | High | Low |
| Hot-reloading | No (native) | No | No | Built-in, transparent |
| Effect system | No | No | No | Built-in |
| Direct C interop | ctypes (painful) | CGo (limited) | FFI (manual) | Header import (automatic) |
| Contracts | No | No | No | Built-in |
| Pipe operator | No | No | No | Built-in |

---

## The Vision

Nova exists because we believe the industry has accepted a false trade-off: either your language is easy to write (Python, JS) or it's fast (Rust, C). Nova rejects that trade-off.

You should be able to write code that reads like Python, runs like Rust, and reloads like Erlang — without learning three different paradigms to get there.

That's Nova.

---

*Nova is open source under the MIT license.*
*Repository: [github.com/Elchi-dev/nova](https://github.com/Elchi-dev/nova)*

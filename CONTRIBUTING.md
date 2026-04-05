# Contributing to Nova

Thanks for your interest in Nova! The language is in early development,
so contributions of all kinds are welcome — bug reports, fixes, docs, and ideas.

---

## Getting started

```bash
git clone https://github.com/Elchi-dev/nova.git
cd nova
cargo check
cargo test --all
```

Requires Rust 1.75+.

---

## Before you submit a PR

Nova enforces clean code via CI. Run these locally before pushing — the CI
will reject anything that fails:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

All three must pass with zero errors or warnings.

---

## Commit style

Nova uses conventional commits:

```
feat: add type inference for let bindings
fix: correct indent tracking for nested blocks
refactor: split parser into expression and statement modules
docs: document @dist decorator semantics
test: add parser tests for function declarations
chore: bump logos to 0.15
style: apply rustfmt
perf: reduce allocations in lexer hot path
```

Keep the subject line under 72 characters. Add a body if the change needs
more context.

---

## Project structure

```
crates/nova-lexer/      Tokenizer — touch this for new tokens or keywords
crates/nova-parser/     Parser + AST — touch this for grammar changes
crates/nova-compiler/   Pipeline orchestration
crates/nova-cli/        CLI subcommands
examples/               Nova source examples (add one for every new feature)
```

---

## Adding a new keyword

1. Add the token to `crates/nova-lexer/src/token.rs`
2. Add a lex test in `crates/nova-lexer/src/tests.rs`
3. Add the AST node in `crates/nova-parser/src/ast.rs` if needed
4. Add a parse rule in `crates/nova-parser/src/parser.rs`
5. Add a parse test in `crates/nova-parser/src/tests.rs`
6. Update `examples/` and `README.md` if the feature is user-facing

---

## Opening issues

- **Bug**: include a minimal `.nv` snippet that reproduces it
- **Feature request**: describe the problem it solves, not just the solution
- **Question**: feel free to open a discussion

---

## License

By contributing you agree that your changes will be licensed under the
[MIT License](LICENSE).

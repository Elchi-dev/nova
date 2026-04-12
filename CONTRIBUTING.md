# Contributing to Nova

Thanks for your interest in Nova! This project is in early development and we welcome contributions of all kinds.

## Ways to Contribute

### Language Design
Open an issue tagged `design` to propose or discuss syntax, semantics, or new features. Nova's design is still evolving — your input matters.

### Compiler Development
The compiler is written in Rust. Areas that need work:
- **Type checker** — Hindley-Milner inference, generic resolution
- **Semantic analysis** — Escape analysis, effect tracking, module splitting
- **Code generation** — LLVM IR output via `inkwell`
- **CLI tools** — Formatter, test runner, REPL

### Documentation
Help explain Nova to the world. Write tutorials, improve docs, or create examples.

### Testing
Write Nova programs and report what breaks. Edge cases in the parser and lexer are especially valuable.

## Development Setup

```bash
# Clone and build
git clone https://github.com/Elchi-dev/nova.git
cd nova
cargo build

# Run tests
cargo test

# Run a specific crate's tests
cargo test -p nova-compiler
cargo test -p nova-runtime
```

## Code Style

- Follow standard Rust formatting (`cargo fmt`)
- Run `cargo clippy` before submitting
- Write tests for new functionality
- Keep commits focused and well-described

## Commit Messages

We use conventional commits:

```
feat: add type inference for let bindings
fix: handle nested indentation in lexer
docs: update roadmap with Phase 3 details
test: add parser tests for match expressions
refactor: simplify arena allocation path
```

## Pull Requests

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Make your changes with tests
4. Run `cargo test` and `cargo clippy`
5. Submit a PR with a clear description

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

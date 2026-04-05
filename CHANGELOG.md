# Changelog

All notable changes to Nova will be documented here.

---

## [Unreleased]

---

## [v0.1.0] — 2026-04-05

### Added

- Rust workspace with four crates: `nova-lexer`, `nova-parser`, `nova-compiler`, `nova-cli`
- Full token set in `nova-lexer` via `logos` — keywords, types, operators, decorators, literals, significant whitespace
- Complete AST definition in `nova-parser` — functions, modules, structs, imports, `@runtime` config, `@dist`, `@persist`, all expressions and statements
- Compiler pipeline skeleton — lexer → parser → type checker (stub) → codegen (stub)
- `nova` CLI with subcommands: `build`, `run`, `check`, `fmt`, `lint`, `pkg`, `new`
- CI workflow — build + test matrix on Ubuntu, macOS, Windows
- Release workflow — cross-platform binary builds for Linux amd64/arm64, macOS amd64/arm64, Windows amd64

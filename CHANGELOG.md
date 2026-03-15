# Changelog

## 0.0.12

Initial release of syntaqlite — a fast, accurate SQL toolkit for SQLite, built from SQLite's own grammar.

### Highlights

- **Formatter** — opinionated SQL formatter with configurable line width, keyword casing, and semicolons. Supports stdin, files, and glob patterns.
- **Parser** — full SQLite SQL parser producing a concrete syntax tree. Handles all SQLite syntax including CTEs, window functions, upsert clauses, and `RETURNING`.
- **Validator** — semantic analysis with diagnostics for unknown tables, columns, and functions. Supports embedded SQL extraction from Python and TypeScript.
- **Language Server (LSP)** — diagnostics, formatting, completions (keywords, tables, columns, functions), and semantic tokens over stdio.
- **WASM / JS** — browser-ready builds powering the interactive playground.
- **Dialect extensibility** — load custom grammars as shared libraries at runtime.

### Install

- CLI binaries for macOS (arm64, x86_64), Linux (arm64, x86_64), and Windows (x86_64)
- `pip install syntaqlite` — bundled platform-specific binary, includes MCP server via `syntaqlite[mcp]`
- `brew install LalitMaganti/tap/syntaqlite`
- `cargo install syntaqlite-cli`
- `mise use github:LalitMaganti/syntaqlite`
- Self-downloading script: `curl ... | python3 -` with weekly auto-updates
- VS Code extension with bundled LSP (VS Code Marketplace + Open VSX)
- Rust crates: `syntaqlite`, `syntaqlite-cli`, `syntaqlite-syntax`, `syntaqlite-common`
- NPM package: `@syntaqlite/js`

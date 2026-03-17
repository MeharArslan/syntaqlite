# Changelog

## 0.1.0

Initial release of syntaqlite — a fast, accurate SQL toolkit for SQLite, built from SQLite's own grammar.

### Highlights

- **Formatter** — opinionated SQL formatter with configurable line width, keyword casing, and semicolons. Supports stdin, files, and glob patterns.
- **Parser** — full SQLite SQL parser producing a concrete syntax tree. Handles all SQLite syntax including CTEs, window functions, upsert clauses, and `RETURNING`.
- **Validator** — semantic analysis with diagnostics for unknown tables, columns, and functions. Supports embedded SQL extraction from Python and TypeScript.
- **Language Server (LSP)** — diagnostics, formatting, completions, go-to-definition, document highlights, and semantic tokens over stdio.
- **C API** — prebuilt shared libraries for macOS, Linux, and Windows, plus a source amalgamation for embedding.
- **WASM / JS** — browser-ready builds powering the interactive playground.
- **Dialect extensibility** — load custom grammars as shared libraries at runtime.

### Project configuration

`syntaqlite.toml` is the single, editor-agnostic source of truth for schemas and formatting — it works across VS Code, Claude Code, Neovim, Helix, and the CLI with no additional setup.

```toml
[schemas]
"src/**/*.sql" = ["schema/main.sql"]
"tests/**/*.sql" = ["schema/main.sql", "schema/test_fixtures.sql"]
"migrations/*.sql" = []

[format]
line-width = 100
keyword-case = "lower"
```

### Install

- CLI binaries for macOS (arm64, x86_64), Linux (arm64, x86_64), and Windows (x86_64, arm64)
- `pip install syntaqlite` — bundled platform-specific binary, includes built-in MCP server (`syntaqlite mcp`)
- `brew install LalitMaganti/tap/syntaqlite`
- `cargo install syntaqlite-cli`
- `mise use github:LalitMaganti/syntaqlite`
- VS Code extension with bundled LSP (VS Code Marketplace + Open VSX)
- Claude Code plugin via Marketplace
- Rust crates: `syntaqlite`, `syntaqlite-cli`, `syntaqlite-syntax`, `syntaqlite-common`
- NPM package: `syntaqlite`

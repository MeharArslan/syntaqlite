# Changelog

## 0.0.26

- Fix config file discovery with relative paths — `discover()` now resolves to absolute paths before walking up, fixing cases where `syntaqlite.toml` in the project root wasn't found
- Add `syntaqlite.toml` to the syntaqlite repo itself (dogfooding)
- Fix Claude Code plugin install instructions (two-step marketplace add + install)
- Add 5 integration tests for config file: schema resolution, glob routing, format options, CLI override, nearest-config-wins
- Documentation: rewrite all getting-started pages as guided walkthroughs, split Claude Code and MCP into separate pages, add config file reference page

## 0.0.25

### Project configuration file

syntaqlite now reads project settings from a `syntaqlite.toml` file. This is the single, editor-agnostic source of truth for schemas and formatting — it works across VS Code, Claude Code, Neovim, Helix, and the CLI with no additional setup.

```toml
[schemas]
"src/**/*.sql" = ["schema/main.sql"]
"tests/**/*.sql" = ["schema/main.sql", "schema/test_fixtures.sql"]
"migrations/*.sql" = []

[format]
line-width = 100
keyword-case = "lower"
```

- **CLI**: `syntaqlite fmt` and `syntaqlite validate` discover `syntaqlite.toml` automatically. CLI flags override config file values.
- **LSP**: The language server reads format config and schema mappings from `syntaqlite.toml` on startup.
- **VS Code extension**: Simplified — `syntaqlite.schemaPath` and `syntaqlite.schemas` settings removed in favor of `syntaqlite.toml`. Only `syntaqlite.serverPath` remains.

### Other changes

- Formatter: improved formatting of RETURNING clauses, WITH/CTE clauses, and window clause ORDER BY
- Formatter: added `--output bytecode` and `--output doc-tree` debug modes
- MCP server: moved from Python to native Rust binary (`syntaqlite mcp`)
- Claude Code plugin: restructured for marketplace, added validate skill
- Documentation: rewrote all getting-started pages as guided walkthroughs, added config file reference page, split Claude Code and MCP docs

## 0.0.24

- Fix license field to Apache-2.0 in VS Code extension and comparison test package
- Fix VS Code extension repository URL

## 0.0.23

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
- `pip install syntaqlite` — bundled platform-specific binary, includes built-in MCP server (`syntaqlite mcp`)
- `brew install LalitMaganti/tap/syntaqlite`
- `cargo install syntaqlite-cli`
- `mise use github:LalitMaganti/syntaqlite`
- Self-downloading script: `curl ... | python3 -` with weekly auto-updates
- VS Code extension with bundled LSP (VS Code Marketplace + Open VSX)
- Rust crates: `syntaqlite`, `syntaqlite-cli`, `syntaqlite-syntax`, `syntaqlite-common`
- NPM package: `syntaqlite`

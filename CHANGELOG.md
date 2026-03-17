# Changelog

## 0.2.0

### Column lineage

- New lineage analysis for SELECT statements — traces each result column back to its source table and column, resolving through CTEs, subqueries, and aliases.
- Three new methods on `SemanticModel`: `lineage()`, `relations_accessed()`, and `tables_accessed()`, returning `Complete` or `Partial` results depending on view resolution.
- C API: 7 new lineage accessor functions (`syntaqlite_validator_column_lineage`, `syntaqlite_validator_relations`, `syntaqlite_validator_tables`, etc.).
- Python API: `validate()` now returns a `ValidationResult` with `.diagnostics` and `.lineage` attributes instead of a raw list of dicts.
- Python API: new `Table`, `View`, and `schema_ddl` parameter for schema registration with proper attribute access.

### Schema registration

- C API: new `syntaqlite_validator_add_views()` for registering views separately from tables.
- C API: new `syntaqlite_validator_load_schema_ddl()` to register schema from DDL strings (CREATE TABLE/VIEW).
- C API: `SyntaqliteTableDef` renamed to `SyntaqliteRelationDef` (used for both tables and views).

### Bug fixes

- Fix stack overflow in lineage resolver for recursive CTEs on Linux (caused 40 upstream test file crashes).
- Fix formatter macro handling to respect per-dialect macro style settings.

### Other

- Python C extension now ships in PyPI wheels (no build step needed).

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

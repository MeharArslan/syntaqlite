# Changelog

## 0.0.34

- C API: prebuilt shared library download (`syntaqlite-clib.tar.gz`) with binaries for macOS, Linux, and Windows plus a single `syntaqlite.h` header
- C API: renamed source amalgamation archive to `syntaqlite-syntax-amalgamation.tar.gz`
- Docs: restructured to Diataxis — Tutorials, Guides, Concepts, Reference, Contributing
- Docs: new Rust library and C parser tutorials
- Docs: mobile-friendly layout with responsive navigation
- Docs: expanded C API guide and reference with parser/tokenizer coverage
- Download script: fix rate limiting on GitHub Actions (now supports `GITHUB_TOKEN`)

## 0.0.33

*Intermediate release — changes rolled into 0.0.34.*

## 0.0.32

*Intermediate release — changes rolled into 0.0.34.*

## 0.0.31

- Docs: mobile-friendly redesign with consistent syntax highlighting
- Fix VS Code extension README link

## 0.0.30

- LSP: go-to-definition now returns `LocationLink` with `originSelectionRange`, giving editors precise control over which token gets underlined on Ctrl+hover
- LSP: add `textDocument/documentHighlight` — highlights all occurrences of a symbol (table, column, CTE) in the current file when the cursor is on it
- CLI: add `--no-config` flag to disable automatic `syntaqlite.toml` discovery
- Fix LSP integration tests that were broken by the repo's own `syntaqlite.toml` interfering with test schema loading

## 0.0.29

- LSP: per-file schema resolution — `[schemas]` glob entries in `syntaqlite.toml` are now respected by the language server. Each open file is matched against glob patterns to select its schema catalog, with `strict_schema` applied automatically for matched files. Previously only the top-level `schema` key was read, so projects using `[schemas]` got no schema validation in the editor.
- Check levels: replace boolean enable/disable with three-level `allow`/`warn`/`deny` per diagnostic category. CLI flags `--enable`/`--disable` replaced by `-A` (allow), `-W` (warn), `-D` (deny). Config file `[checks]` values are now strings (`"allow"`, `"warn"`, `"deny"`) instead of booleans.

## 0.0.28

- Fix VS Code extension crash: `workspaceRoot` was referenced before declaration (TDZ error), silently preventing the extension from activating

## 0.0.27

- Add global `--config` flag to explicitly pass `syntaqlite.toml` path (VS Code extension uses this to avoid cwd dependency)
- Schema-aware diagnostic severity: with a schema (`--schema` or `syntaqlite.toml`), unresolved names are errors (exit 1); without, they are warnings (exit 0)
- "No schema provided" hint printed to stderr when running validate without any schema source
- VS Code extension passes `--config` to LSP server when `syntaqlite.toml` exists in workspace root
- Documentation: add schema validation guide, document severity behavior in CLI reference

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

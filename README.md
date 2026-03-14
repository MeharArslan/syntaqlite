# syntaqlite

[![CI](https://github.com/LalitMaganti/syntaqlite/actions/workflows/ci.yml/badge.svg)](https://github.com/LalitMaganti/syntaqlite/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/syntaqlite)](https://crates.io/crates/syntaqlite)
[![VS Code](https://img.shields.io/visual-studio-marketplace/v/syntaqlite.syntaqlite)](https://marketplace.visualstudio.com/items?itemName=syntaqlite.syntaqlite)

A parser, formatter, validator, and language server for SQLite SQL — built directly from SQLite's own tokenizer and grammar rules. If SQLite accepts it, syntaqlite parses it identically.

**[Docs](https://docs.syntaqlite.com)** · **[Playground](https://playground.syntaqlite.com)** · **[VS Code Extension](https://marketplace.visualstudio.com/items?itemName=syntaqlite.syntaqlite)** · **[MCP Server](integrations/mcp/README.md)**

## Why syntaqlite

Most SQL tools parse a subset of SQL, invent their own grammar, or handle SQLite as an afterthought. syntaqlite uses SQLite's own tokenizer and grammar rules directly — the parser doesn't approximate SQLite, it _is_ SQLite's grammar compiled into a reusable library.

Every quirk, every edge case, every syntax extension that SQLite supports works correctly from day one. CTEs, window functions, upsert, `RETURNING`, generated columns, `WITHOUT ROWID` — if SQLite parses it, syntaqlite parses it.

syntaqlite grew out of 8+ years of maintaining [PerfettoSQL](https://perfetto.dev/docs/analysis/perfetto-sql-syntax) and scaling it to 100K+ line SQL codebases where generic SQL tooling consistently falls short.

## Features

- **Format** — consistent SQL style across a project; Wadler-Lindig pretty-printing that understands SQLite syntax
- **Validate** — catch unknown tables, columns, and functions before runtime, not after your app ships
- **Parse** — full parse trees for code generation, migration tooling, or static analysis
- **LSP** — diagnostics, completions, and formatting in any editor
- **WASM** — runs in the browser; powers the [interactive playground](https://playground.syntaqlite.com)
- **Version-aware** — pin to a specific SQLite version or enable compile-time flags to match your exact build

## Install

**Homebrew**

```bash
brew install LalitMaganti/tap/syntaqlite
```

**Shell (macOS / Linux)**

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/LalitMaganti/syntaqlite/releases/latest/download/syntaqlite-cli-installer.sh | sh
```

**PowerShell (Windows)**

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/LalitMaganti/syntaqlite/releases/latest/download/syntaqlite-cli-installer.ps1 | iex"
```

**Cargo**

```bash
cargo install syntaqlite-cli
```

## Quick start

```bash
# Format SQL (inline expression)
syntaqlite fmt -e "SELECT a,b FROM t WHERE x=1"

# Format SQL (stdin)
echo "SELECT a,b FROM t WHERE x=1" | syntaqlite fmt

# Format a file in place
syntaqlite fmt -i query.sql

# Format with options (line width, keyword casing)
syntaqlite fmt -w 120 -k upper query.sql

# Parse and inspect the AST
syntaqlite parse -e "SELECT 1 + 2" --output ast

# Validate SQL — catch unknown tables, columns, functions
syntaqlite validate schema.sql

# Validate embedded SQL in Python or TypeScript source (experimental)
syntaqlite validate --experimental-lang python app.py
```

## SQLite version and flag support

SQLite isn't one fixed language — syntax changes between releases, and compile-time flags enable optional features. syntaqlite tracks this: pin the parser to a specific SQLite version, or enable flags to match your exact build.

```bash
# Parse as SQLite 3.47.0 (reject syntax added in later versions)
syntaqlite fmt --sqlite-version 3.47.0 query.sql

# Enable optional syntax from compile-time flags
syntaqlite validate --sqlite-cflag SQLITE_ENABLE_ORDERED_SET_AGGREGATES query.sql
```

## Editor integration

**VS Code** — install the [syntaqlite extension](https://marketplace.visualstudio.com/items?itemName=syntaqlite.syntaqlite) from the marketplace. Provides diagnostics, formatting, and completions out of the box.

**Claude Code** — install the plugin:

```
/plugin install syntaqlite
```

**Claude Desktop / Cursor** — install the MCP server:

```bash
pip install syntaqlite-mcp
```

See the [MCP server docs](integrations/mcp/README.md) for per-client configuration.

**Any editor with LSP support** — point your editor at the language server:

```bash
syntaqlite lsp
```

## Use as a library

**Rust**

```toml
[dependencies]
syntaqlite = { version = "0.1", features = ["fmt"] }
```

**JavaScript / WASM**

```bash
npm install @syntaqlite/js
```

**C** — the parser, tokenizer, formatter, and validator all have C APIs. See the [C API docs](https://docs.syntaqlite.com/reference/c-api/) for details.

## Architecture

The parser and tokenizer are written in C, directly wrapping SQLite's own grammar. Everything else — formatter, validator, LSP — is written in Rust with C bindings available.

The split is intentional. The C parser is as portable as SQLite itself: it can run inside database engines, embedded systems, or anywhere SQLite runs. The Rust layer moves fast for developer tooling where the standard library and the crate ecosystem matter.

## Building from source

```bash
tools/install-build-deps
tools/cargo build
```

## Contributing

See the [contributing guide](https://docs.syntaqlite.com/contributing/) for architecture overview and testing instructions.

## License

Apache 2.0. SQLite components are public domain under the [SQLite blessing](https://www.sqlite.org/copyright.html). See [LICENSE](LICENSE) for details.

# syntaqlite: syntatic tools for SQLite

Suite of developer oriented libraries and tools for working with SQLite SQL: tokenizer, parser, formatter, validator, and language server (LSP).

There are [many](https://sqlglot.com/sqlglot.html) [libraries](https://www.sqlfluff.com/) [out](https://github.com/apache/datafusion-sqlparser-rs)
which do some/most/all of what syntaqlite does. So why decide to write _yet another_ library instead of contributing to an existing one?

Fundamentally, the fundamental design principles of syntaqlite is _very different_ to anything I could find:

Most SQL tools parse a subset of SQL, or invent their own grammar, or handle SQLite as an afterthought. syntaqlite takes a different approach: it uses SQLite's own tokenizer and grammar rules directly. The parser doesn't approximate SQLite — it _is_ SQLite's grammar, compiled into a reusable library.

This means every quirk, every edge case, every syntax extension that SQLite supports works correctly from day one. CTEs, window functions, upsert, `RETURNING`, generated columns, `WITHOUT ROWID` — if SQLite parses it, syntaqlite parses it identically.

## Why

syntaqlite grew out of 8+ years of maintaining [PerfettoSQL](https://perfetto.dev/docs/analysis/perfetto-sql-syntax) and scaling it to 100K+ line SQL codebases where generic SQL tooling consistently falls short. We needed a foundation that didn't just "mostly work" — it had to be identical to the engine.

If you write SQL for SQLite, you've probably hit one of these:

- **Formatting** — you want consistent SQL style across a project, but generic SQL formatters mangle SQLite-specific syntax or produce output that doesn't feel like SQL at all.
- **Validation** — you want to catch typos in table and column names before runtime, not after your app ships.
- **Parsing** — you need a real parse tree for code generation, migration tooling, or static analysis — not a regex or a half-working grammar.
- **Editor support** — you want diagnostics, completions, and formatting in your editor, but existing SQL extensions don't understand SQLite well.

syntaqlite solves all of these with a single foundation: SQLite's own grammar.

## Design principles

- **Reliability** — uses SQLite's own tokenizer and grammar rules; verified by running the full SQLite test suite through the parser. <!-- TODO: add XX% parity number -->
- **Speed** — zero-copy tokenizer, arena-allocated parser, reusable across inputs. The formatter is built on Wadler-Lindig pretty-printing.
- **Portability** — no runtime dependencies beyond the C and Rust standard libraries. Runs natively, in WASM, and as a shared library.
- **Extensibility** — the grammar system supports database engines that extend SQLite's syntax (like [PerfettoSQL](https://perfetto.dev/docs/analysis/perfetto-sql-syntax)'s `CREATE PERFETTO MACRO`). Define custom grammar rules, AST nodes, and formatting recipes, then load your dialect as a shared library at runtime.

## SQLite version and flag support

SQLite isn't one fixed language — syntax changes between releases, and compile-time flags enable optional features. syntaqlite tracks this: pin the parser to a specific SQLite version, or enable compile-time flags to match your exact build.

```bash
# Parse as SQLite 3.47.0 (reject syntax added in later versions)
syntaqlite fmt --sqlite-version 3.47.0 query.sql

# Enable optional syntax from compile-time flags
syntaqlite validate --sqlite-cflag SQLITE_ENABLE_ORDERED_SET_AGGREGATES query.sql
```

This matters in practice. If your production SQLite is compiled without `SQLITE_ENABLE_MATH_FUNCTIONS`, syntaqlite can flag calls to `sin()` or `log()` as errors — before they reach production.

## Non-goals

- **Other SQL engines** — syntaqlite is SQLite-only, by design. The depth of integration with SQLite's grammar is what makes it reliable; trying to also handle PostgreSQL or MySQL would undermine that.
- **Runtime errors** — syntaqlite catches what `sqlite3_prepare` would catch: syntax errors, unknown tables, unknown columns, unknown functions. It does not try to catch data-dependent errors like division by zero or type mismatches — SQLite's dynamic typing makes that largely impossible to do statically.

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
# Format SQL
echo "SELECT a,b FROM t WHERE x=1" | syntaqlite fmt

# Format a file in place
syntaqlite fmt -i query.sql

# Format with options (line width, keyword casing)
syntaqlite fmt -w 120 -k upper query.sql

# Parse and inspect the AST
echo "SELECT 1 + 2" | syntaqlite ast

# Validate SQL — catch unknown tables, columns, functions
syntaqlite validate schema.sql

# Validate embedded SQL in Python or TypeScript source (experimental)
syntaqlite validate --experimental-lang python app.py
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

## Embedded language support (experimental)

SQL lives inside other languages — Python strings, TypeScript template literals, query builders. syntaqlite can extract SQL from host language source files and provide validation, diagnostics, and LSP support directly inside those strings, including best-effort handling of template interpolations.

Python and TypeScript are supported today.

```bash
syntaqlite validate --experimental-lang python app.py
```

## Architecture

The parser and tokenizer are written in C, directly wrapping SQLite's own grammar. Everything else — formatter, validator, LSP — is written in Rust with C bindings available.

The split is intentional. The C parser is as portable as SQLite itself: it can run inside database engines, embedded systems, or anywhere SQLite runs. The Rust layer moves fast for developer tooling where the standard library and the crate ecosystem matter.

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

**C** — the parser, tokenizer, formatter, and validator all have C APIs. See the [C API documentation](TODO) for details.

## Try it

The [interactive playground](https://lalitmaganti.github.io/syntaqlite/) runs entirely in your browser via WASM — no install needed.

## Building from source

```bash
tools/install-build-deps
tools/cargo build
```

## License

Apache 2.0. SQLite components are public domain under the [SQLite blessing](https://www.sqlite.org/copyright.html). See [LICENSE](LICENSE) for details.

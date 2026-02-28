---
name: syntaqlite
description: Format, parse, and analyze SQLite SQL queries using the syntaqlite CLI. Use when working with .sql files, formatting SQL, inspecting ASTs, or running the syntaqlite language server.
license: Apache-2.0
metadata:
  author: syntaqlite
  version: "0.1.0"
---

# syntaqlite

A fast, accurate SQL formatter and parser for SQLite and SQLite-based dialects. Uses SQLite's own grammar for 100% compatibility.

## When to use this skill

- The user wants to format SQL files
- The user wants to inspect or debug a SQL parse tree (AST)
- The user is working with `.sql` files in a project that uses SQLite
- The user mentions syntaqlite by name

## CLI commands

### Format SQL

```bash
# Format from stdin
echo "SELECT a,b FROM t WHERE x=1" | syntaqlite fmt

# Format a file (print to stdout)
syntaqlite fmt query.sql

# Format in-place
syntaqlite fmt -i query.sql

# Format with options
syntaqlite fmt -w 120 -k upper query.sql

# Format multiple files via glob
syntaqlite fmt "**/*.sql"
```

Options:
- `-w, --line-width <N>` — max line width (default: 80)
- `-k, --keyword-case <upper|lower>` — keyword casing (default: upper)
- `-i, --in-place` — overwrite files in place
- `--semicolons <true|false>` — append semicolons (default: true)

### Parse and inspect AST

```bash
# Print AST from stdin
echo "SELECT 1" | syntaqlite ast

# Print AST from file
syntaqlite ast query.sql
```

### Language server (LSP)

```bash
# Start the LSP server on stdio
syntaqlite lsp
```

The LSP provides: diagnostics, formatting, completions (keywords, tables, columns, functions), and semantic tokens.

## Installation

Build from source with Cargo:

```bash
cargo install --path syntaqlite-cli
```

The binary is named `syntaqlite`.

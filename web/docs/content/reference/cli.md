+++
title = "CLI reference"
description = "All flags and options for every subcommand."
weight = 1
+++

# CLI reference

## syntaqlite fmt

Format SQL files.

```
syntaqlite fmt [OPTIONS] [FILES...]
```

Reads from stdin if no files are given.

| Option | Default | Description |
|--------|---------|-------------|
| `-w, --line-width <N>` | `80` | Target maximum line width |
| `-k, --keyword-case <CASE>` | `upper` | Keyword casing: `upper` or `lower` |
| `--semicolons <BOOL>` | `true` | Append semicolons after statements |
| `-i, --in-place` | | Write formatted output back to files |
| `--dialect <PATH>` | | Path to custom dialect shared library |
| `--dialect-name <NAME>` | | Symbol name in dialect library |
| `--sqlite-version <VER>` | `latest` | Target SQLite version (e.g., `3.47.0`) |
| `--sqlite-cflag <FLAG>` | | Enable a compile-time flag (repeatable) |

Exit codes:
- `0` — success
- `1` — parse error

## syntaqlite validate

Validate SQL against schema.

```
syntaqlite validate [OPTIONS] [FILES...]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--experimental-lang <LANG>` | | Extract SQL from `python` or `typescript` source |
| `--dialect <PATH>` | | Path to custom dialect shared library |
| `--dialect-name <NAME>` | | Symbol name in dialect library |
| `--sqlite-version <VER>` | `latest` | Target SQLite version |
| `--sqlite-cflag <FLAG>` | | Enable a compile-time flag (repeatable) |

The validator builds schema from `CREATE TABLE` / `CREATE VIEW` statements in
the input, then checks queries against that schema. Diagnostics are printed to
stderr in rustc-style format.

## syntaqlite ast

Print the parsed AST.

```
syntaqlite ast [FILES...]
```

Reads from stdin if no files are given. When multiple files are provided, each
is prefixed with `==> filename <==`.

Exit codes:
- `0` — parsed successfully
- `1` — parse error

## syntaqlite lsp

Start the language server on stdio.

```
syntaqlite lsp [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--dialect <PATH>` | | Path to custom dialect shared library |
| `--dialect-name <NAME>` | | Symbol name in dialect library |
| `--sqlite-version <VER>` | `latest` | Target SQLite version |
| `--sqlite-cflag <FLAG>` | | Enable a compile-time flag (repeatable) |

Supports:
- `textDocument/publishDiagnostics`
- `textDocument/formatting`
- `textDocument/completion`
- `textDocument/semanticTokens/full`

## Global options

These options are available on all subcommands:

| Option | Description |
|--------|-------------|
| `--dialect <PATH>` | Load a custom dialect from a shared library |
| `--dialect-name <NAME>` | Symbol name for the dialect (default: `syntaqlite_grammar`, with name: `syntaqlite_<NAME>_grammar`) |
| `--sqlite-version <VER>` | Emulate a specific SQLite version |
| `--sqlite-cflag <FLAG>` | Enable a SQLite compile-time flag (can be specified multiple times) |

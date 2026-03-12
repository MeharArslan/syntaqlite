+++
title = "Custom dialects"
description = "Define and load a grammar extension for non-standard SQLite syntax."
weight = 3
+++

# Custom dialects

syntaqlite's grammar is extensible. If you have a SQL dialect that adds syntax
on top of SQLite — custom statements, additional functions, new clauses — you
can define a dialect that syntaqlite will use for parsing, formatting, and
validation.

## How dialects work

A dialect is a shared library (`.so`, `.dylib`, or `.dll`) that provides:

- A grammar extending SQLite's parser rules
- AST node definitions for new syntax
- Formatting rules for the new nodes
- Semantic role annotations for validation

syntaqlite loads the dialect at runtime using the `--dialect` flag:

```bash
syntaqlite fmt --dialect /path/to/libmydialect.so query.sql
syntaqlite validate --dialect /path/to/libmydialect.so query.sql
syntaqlite lsp --dialect /path/to/libmydialect.so
```

## Defining a dialect

Dialects are defined using `.synq` files — the same grammar definition language
syntaqlite uses internally. A `.synq` file declares nodes, enums, formatting
rules, and semantic annotations.

Here's a minimal example from the Perfetto dialect, which adds a
`CREATE PERFETTO MACRO` statement:

```synq
node CreatePerfettoMacroStmt {
  name: index SqliteIdent
  args: index PerfettoMacroArgList
  returns: inline PerfettoMacroReturns
  body: index Expr

  fmt {
    group {
      "CREATE PERFETTO MACRO" line
      child(name)
      "(" nest { softline child(args) } softline ")"
      line "RETURNS" " " child(returns)
      line "AS" nest { line child(body) }
    }
  }
}
```

## Building a dialect

Use the `syntaqlite dialect` command to generate C sources and Rust bindings
from your `.synq` definitions:

```bash
syntaqlite dialect --name mydialect --nodes-dir path/to/nodes --output-dir generated/
```

Then compile the generated sources into a shared library. See the
[Perfetto dialect](https://github.com/LalitMaganti/syntaqlite/tree/main/dialects/perfetto)
in the repository for a complete working example.

## Dialect naming

By default, syntaqlite looks for a symbol called `syntaqlite_grammar` in the
shared library. If your dialect has a specific name, use `--dialect-name`:

```bash
syntaqlite fmt --dialect /path/to/lib.so --dialect-name perfetto query.sql
```

This looks for the symbol `syntaqlite_perfetto_grammar`.

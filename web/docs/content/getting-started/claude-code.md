+++
title = "Claude Code"
description = "Install the syntaqlite plugin for Claude Code."
weight = 2
+++

# Claude Code

syntaqlite has a Claude Code plugin that gives Claude access to SQL formatting,
parsing, and validation tools.

## Install the plugin

```bash
claude plugin install syntaqlite
```

Once installed, Claude can format SQL, inspect parse trees, and run the
language server — you can ask it to format a query, check a `.sql` file for
errors, or debug a parse issue.

To configure schemas and formatting for your project, create a
[`syntaqlite.toml`](@/reference/config-file.md) in your project root — the LSP
reads it automatically.

## What the plugin provides

- **Format SQL** — `syntaqlite fmt` with configurable line width, keyword
  casing, and semicolons
- **Parse SQL** — `syntaqlite parse` to inspect the parse tree
- **Language server** — `syntaqlite lsp` for diagnostics, completions, and
  semantic tokens

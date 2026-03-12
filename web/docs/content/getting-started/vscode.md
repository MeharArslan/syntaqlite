+++
title = "VS Code"
description = "Install the extension — diagnostics, formatting, and completions out of the box."
weight = 1
+++

# VS Code

Install the **syntaqlite** extension from the
[VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=syntaqlite.syntaqlite).
It bundles the syntaqlite binary for your platform — no separate install
needed.

Open any `.sql` file and you get:

- **Diagnostics** — syntax errors and semantic warnings as you type
- **Format on save** — or run "Format Document" manually
- **Completions** — SQL keywords, built-in functions, table and column names
- **Semantic highlighting** — context-aware syntax coloring (keywords, strings,
  identifiers, etc. colored by meaning, not just pattern matching)

That's it. There's nothing to configure for basic use.

## Configuration

The extension has one setting:

| Setting | Default | Description |
|---------|---------|-------------|
| `syntaqlite.serverPath` | `""` | Absolute path to the syntaqlite binary. Leave empty to use the bundled binary or PATH. |

This is useful if you're developing syntaqlite itself or want to use a
build with specific features enabled.

## Commands

Open the command palette (`Cmd+Shift+P` / `Ctrl+Shift+P`):

- **syntaqlite: Restart Language Server** — restart after a crash or config
  change
- **syntaqlite: Format Document** — format the current file

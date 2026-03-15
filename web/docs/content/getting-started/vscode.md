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

Open any `.sql` file. You'll see:

- Syntax errors underlined as you type
- Format on save (or run "Format Document" from the command palette)
- Keyword and function completions
- Semantic syntax coloring

To enable table and column validation, create a
[`syntaqlite.toml`](@/reference/config-file.md) in your project root pointing
at your schema DDL files.

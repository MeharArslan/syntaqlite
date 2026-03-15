+++
title = "Other editors"
description = "Neovim, Helix, or any editor with LSP support."
weight = 4
+++

# Other editors

syntaqlite ships a language server that works with any editor that supports LSP.

## Neovim

Add this to your Neovim config (requires
[nvim-lspconfig](https://github.com/neovim/nvim-lspconfig)):

```lua
vim.lsp.config('syntaqlite', {
  cmd = { 'syntaqlite', 'lsp' },
  filetypes = { 'sql' },
  root_markers = { 'syntaqlite.toml', '.git' },
})
vim.lsp.enable('syntaqlite')
```

Open a `.sql` file. You should see syntax errors underlined, and formatting
works via your usual LSP format keybinding.

## Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "sql"
language-servers = ["syntaqlite"]

[language-server.syntaqlite]
command = "syntaqlite"
args = ["lsp"]
```

Restart Helix and open a `.sql` file. Diagnostics appear inline and `:format`
formats the buffer.

## Other LSP clients

The pattern is the same for any editor: set the server command to
`syntaqlite lsp` and associate it with SQL files. The server communicates over
stdin/stdout using JSON-RPC.

## Add schema validation

Without a schema, syntaqlite validates against an empty catalog — syntax errors
and built-in function checks work, but unknown tables and columns won't be
caught. To enable full validation, set up a schema file — see the
[schema validation guide](@/guides/schema-validation.md) for instructions.

Restart the language server (or reopen the editor) after adding the config.
Queries referencing unknown columns or tables will show warnings with "did you
mean?" suggestions.

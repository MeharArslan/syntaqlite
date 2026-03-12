+++
title = "Other editors"
description = "Neovim, Helix, or any editor with LSP support."
weight = 4
+++

# Other editors

syntaqlite implements the Language Server Protocol. Any editor with LSP support
can use it.

## Setup

Start the language server:

```bash
syntaqlite lsp
```

This runs on stdio. Point your editor's LSP client at this command.

The server supports:

- `textDocument/publishDiagnostics` — syntax and semantic errors
- `textDocument/formatting` — format document or range
- `textDocument/completion` — keywords, functions, table/column names
- `textDocument/semanticTokens/full` — context-aware highlighting

## Neovim

With [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig):

```lua
vim.lsp.config('syntaqlite', {
  cmd = { 'syntaqlite', 'lsp' },
  filetypes = { 'sql' },
  root_markers = { '.git' },
})
vim.lsp.enable('syntaqlite')
```

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

## Other editors

The pattern is the same for any LSP client: set the command to
`syntaqlite lsp` and associate it with SQL files. The server communicates over
stdin/stdout using JSON-RPC.

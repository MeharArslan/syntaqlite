+++
title = "Claude Code"
description = "Plugin and MCP server for Claude Code, Claude Desktop, Cursor, and Windsurf."
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
reads it automatically with no additional setup needed.

## What the plugin provides

- **Format SQL** — `syntaqlite fmt` with configurable line width, keyword
  casing, and semicolons
- **Parse SQL** — `syntaqlite parse` to inspect the parse tree
- **Language server** — `syntaqlite lsp` for diagnostics, completions, and
  semantic tokens

## MCP server

If you use Claude Desktop, Cursor, Windsurf, or other MCP-compatible tools,
you can also set up syntaqlite as an MCP server. This exposes three tools:
`format_sql`, `parse_sql`, and `validate_sql`.

### Install

The MCP server is built into the `syntaqlite` binary — no extra dependencies
needed. Install via any method:

```bash
# Download script (all platforms, recommended)
curl -sSf https://raw.githubusercontent.com/LalitMaganti/syntaqlite/main/tools/syntaqlite | python3 - install

# Homebrew (macOS)
brew install LalitMaganti/tap/syntaqlite

# Cargo
cargo install syntaqlite-cli

# pip
pip install syntaqlite

# mise
mise use github:LalitMaganti/syntaqlite
```

See the [CLI install docs](@/getting-started/cli.md) for all options.

### Claude Desktop

Add to your config file:

- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Linux: `~/.config/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "syntaqlite": {
      "command": "syntaqlite",
      "args": ["mcp"]
    }
  }
}
```

### Cursor

Add to `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "syntaqlite": {
      "command": "syntaqlite",
      "args": ["mcp"]
    }
  }
}
```

### Windsurf

Add to `~/.codeium/windsurf/mcp_config.json`:

```json
{
  "mcpServers": {
    "syntaqlite": {
      "command": "syntaqlite",
      "args": ["mcp"]
    }
  }
}
```

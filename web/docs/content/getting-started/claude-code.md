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

## What the plugin provides

- **Format SQL** — `syntaqlite fmt` with configurable line width, keyword
  casing, and semicolons
- **Parse SQL** — `syntaqlite ast` to inspect the parse tree
- **Language server** — `syntaqlite lsp` for diagnostics, completions, and
  semantic tokens

## MCP server

If you use Claude Desktop, Cursor, Windsurf, or other MCP-compatible tools,
you can also set up syntaqlite as an MCP server. This exposes three tools:
`format_sql`, `parse_sql`, and `validate_sql`.

### Install

```bash
pip install syntaqlite-mcp
```

The `syntaqlite` CLI must be on your `PATH`.

### Claude Desktop

Add to your config file:

- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Linux: `~/.config/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "syntaqlite": {
      "command": "syntaqlite-mcp"
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
      "command": "syntaqlite-mcp"
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
      "command": "syntaqlite-mcp"
    }
  }
}
```

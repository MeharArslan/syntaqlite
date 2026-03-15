+++
title = "MCP server"
description = "Set up the syntaqlite MCP server for Claude Desktop, Cursor, and Windsurf."
weight = 5
+++

# MCP server

syntaqlite includes an MCP server that exposes `format_sql`, `parse_sql`, and
`validate_sql` tools. Use it with Claude Desktop, Cursor, Windsurf, or any
MCP-compatible client.

## Install

The MCP server is built into the `syntaqlite` binary. Install it first:

```bash
# Download script (all platforms)
curl -sSf https://raw.githubusercontent.com/LalitMaganti/syntaqlite/main/tools/syntaqlite | python3 - install
```

See the [CLI install docs](@/getting-started/cli.md) for all install methods.

## Claude Desktop

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

## Cursor

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

## Windsurf

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

## Available tools

See the [MCP tools reference](@/reference/mcp-tools.md) for parameters and
examples.

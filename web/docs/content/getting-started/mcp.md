+++
title = "MCP server"
description = "Set up the syntaqlite MCP server for Claude Desktop, Cursor, and Windsurf."
weight = 5
+++

# MCP server

syntaqlite includes an MCP server for Claude Desktop, Cursor, Windsurf, and
other MCP-compatible clients.

## Install syntaqlite

The MCP server is built into the `syntaqlite` binary. If you haven't installed
it yet:

```bash
curl -sSf https://raw.githubusercontent.com/LalitMaganti/syntaqlite/main/tools/syntaqlite | python3 - install
```

See the [CLI tutorial](@/getting-started/cli.md) for other install methods.

## Claude Desktop

Open your config file:

- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Linux: `~/.config/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`

Add the syntaqlite server:

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

Restart Claude Desktop. You can now ask Claude to format or validate SQL, and
it will use syntaqlite's tools.

## Cursor

Add to `.cursor/mcp.json` in your project:

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

Restart Cursor. Try asking it to format a SQL query — it will use syntaqlite's
`format_sql` tool.

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

Restart Windsurf.

## Try it out

Ask your AI assistant something like:

> Format this SQL: `select id,name from users where active=1`

It should call syntaqlite's `format_sql` tool and return:

```sql
SELECT id, name FROM users WHERE active = 1;
```

See the [MCP tools reference](@/reference/mcp-tools.md) for all available
tools and their parameters.

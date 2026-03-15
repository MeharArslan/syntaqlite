# syntaqlite MCP Server

MCP server exposing syntaqlite SQL tools for use with Claude Desktop, Claude Code, Cursor, Windsurf, and other MCP clients.

## Tools

- **`format_sql`** — Format SQL with configurable line width, keyword casing, and semicolons
- **`parse_sql`** — Return the AST text dump for a SQL string
- **`validate_sql`** — Check for syntax errors; returns `{valid, errors}` JSON

## Install

```sh
pip install syntaqlite[mcp]
```

This installs the `syntaqlite` CLI binary and the `syntaqlite-mcp` MCP server command. You can also run the server as `python -m syntaqlite.mcp`.

## Provider Configuration

### Claude Desktop

Add to your `claude_desktop_config.json`:

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

### Claude Code

Add to `.claude/settings.json` or `~/.claude/settings.json`:

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

### Using uvx (no install needed)

If you have [uv](https://docs.astral.sh/uv/) installed, you can skip `pip install` and point providers directly at uvx:

```json
{
  "mcpServers": {
    "syntaqlite": {
      "command": "uvx",
      "args": ["--extra", "mcp", "syntaqlite-mcp"]
    }
  }
}
```

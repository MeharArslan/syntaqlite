+++
title = "Claude Code"
description = "Install the syntaqlite plugin for Claude Code."
weight = 2
+++

# Claude Code

## Install the plugin

```bash
claude plugin install syntaqlite
```

The plugin starts the syntaqlite language server automatically for `.sql`
files. Open any `.sql` file and you'll see syntax errors underlined, keyword
completions, and formatting via your editor's format command.

## Format a query

Ask Claude to format some SQL:

> Format this: `select id,name from users where active=1 order by name`

Claude uses `syntaqlite fmt` under the hood and returns:

```sql
SELECT id, name
FROM users
WHERE active = 1
ORDER BY name;
```

## Validate a file

If you have a `.sql` file with errors, ask Claude to check it:

> Run syntaqlite validate on query.sql using schema.sql

Claude will show you any unknown tables, columns, or function typos with
suggestions.

## Set up schema validation

Create a `syntaqlite.toml` in your project root so the LSP and CLI
automatically know which schema to use:

```toml
schema = ["schema.sql"]
```

Now the language server provides table and column diagnostics, completions, and
hover info as you edit `.sql` files — and `syntaqlite validate` picks up the
schema without needing `--schema`.

See the [config file reference](@/reference/config-file.md) for glob-based
schema routing and formatting options.

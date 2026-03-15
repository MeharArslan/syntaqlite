+++
title = "Claude Code"
description = "Install the syntaqlite plugin for Claude Code."
weight = 2
+++

# Claude Code

## Install the plugin

```bash
claude plugin marketplace add LalitMaganti/claude-code-plugin
claude plugin install syntaqlite@LalitMaganti/claude-code-plugin
```

The plugin starts the syntaqlite language server for `.sql` files. When Claude
writes or edits SQL, the server feeds back syntax errors, unknown
tables/columns, and function typos — so Claude catches and fixes mistakes
automatically without you having to ask.

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

Create a `syntaqlite.toml` in your project root so the language server knows
which schema to validate against:

```toml
schema = ["schema.sql"]
```

Now when Claude writes SQL that references a column or table that doesn't exist
in your schema, it sees the error immediately and fixes it — the same way a
type checker catches mistakes in code.

See the [config file reference](@/reference/config-file.md) for glob-based
schema routing and formatting options.

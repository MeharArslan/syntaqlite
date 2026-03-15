+++
title = "Claude Code"
description = "Install the syntaqlite plugin for Claude Code."
weight = 2
+++

# Claude Code

## Install the plugin

Add the marketplace and install the plugin:

```bash
claude plugin marketplace add LalitMaganti/claude-code-plugin
claude plugin install syntaqlite@LalitMaganti/claude-code-plugin
```

## What you get

The plugin gives Claude two things:

**Language server** — starts automatically for `.sql` files. When Claude writes
or edits SQL, the server feeds back syntax errors, unknown tables/columns, and
function typos. Claude sees these diagnostics and fixes mistakes on its own,
without you having to ask.

**CLI skills** — Claude can run `syntaqlite fmt`, `syntaqlite validate`, and
`syntaqlite parse` directly when you ask it to format a query, check a file, or
inspect a parse tree.

## Try it out

Open a project with `.sql` files and ask Claude to edit one. If the SQL has an
error — say a misspelled column name — Claude will notice the diagnostic from
the language server and fix it in the same edit.

You can also ask Claude to format or validate explicitly:

> Format all the SQL files in src/

> Check query.sql for errors against schema.sql

## Add schema validation

Without a schema, the language server checks syntax only. To enable table and
column validation, create a `syntaqlite.toml` in your project root:

```toml
schema = ["schema.sql"]
```

Then create `schema.sql` with your table definitions:

```sql
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT);
CREATE TABLE orders (id INTEGER, user_id INTEGER, amount REAL);
```

Now when Claude writes `SELECT nme FROM users`, the language server flags `nme`
as unknown and suggests `name`. Claude sees this and corrects it automatically.

See the [config file reference](@/reference/config-file.md) for glob-based
schema routing and formatting options.

+++
title = "Schema validation"
description = "Set up schema-aware validation for tables, columns, and functions."
weight = 2
+++

# Schema validation

Without a schema, syntaqlite validates against an empty catalog — syntax errors
and built-in function checks still work, but unknown table and column references
won't be caught. To get full validation, tell syntaqlite about your schema.

## Create a project config

Add a `syntaqlite.toml` to your project root:

```toml
schema = ["schema.sql"]
```

The `schema` key accepts a list of file paths or glob patterns. All matched
files are parsed for `CREATE TABLE` and `CREATE VIEW` statements.

## Define your schema

Create `schema.sql` next to `syntaqlite.toml` with your table definitions:

```sql
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT, active INTEGER);
CREATE TABLE posts (id INTEGER, user_id INTEGER, title TEXT, body TEXT);
```

## How it works

The CLI, language server, and WASM engine all read `syntaqlite.toml`
automatically. Once configured:

- The **CLI** (`syntaqlite validate query.sql`) loads the schema before
  validating
- The **language server** picks up the config on startup — diagnostics appear
  in your editor as you type
- **CI** checks work the same way (`syntaqlite validate "**/*.sql"`)

Queries referencing unknown tables, columns, or functions produce warnings with
source locations and "did you mean?" suggestions.

## Passing schema on the command line

For one-off checks without a config file, use `--schema`:

```bash
syntaqlite validate --schema schema.sql query.sql
```

## Next steps

- [Config file reference](@/reference/config-file.md) — glob-based schema
  routing, formatting options, and the full `syntaqlite.toml` format
- [Validation guide](@/guides/validation.md) — embedded SQL, version pinning,
  and Rust API usage

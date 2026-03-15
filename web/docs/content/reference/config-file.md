+++
title = "Config file"
description = "syntaqlite.toml format — schemas, formatting, and discovery."
weight = 3
+++

# Config file

syntaqlite reads project settings from a `syntaqlite.toml` file. This is the
single source of truth for schemas, formatting options, and other project
configuration — it works across every editor and the CLI with no additional
setup.

## Discovery

syntaqlite walks up from the file being processed (or the current directory for
CLI commands) and uses the first `syntaqlite.toml` it finds. This is the same
convention as `rustfmt.toml`, `ruff.toml`, and `.prettierrc`.

## File format

```toml
# Schema DDL files for validation and completions.
# Each entry maps a glob pattern to schema file(s).
# SQL files matching the glob get validated against those schemas.
[schemas]
"src/**/*.sql" = ["schema/main.sql", "schema/views.sql"]
"tests/**/*.sql" = ["schema/main.sql", "schema/test_fixtures.sql"]
"migrations/*.sql" = []  # no schema validation for migrations

# Default schema for SQL files that don't match any glob above.
# Optional — if omitted, unmatched files get no schema.
# schema = ["schema.sql"]

# Formatting options (all optional, shown with defaults).
[format]
line-width = 80
indent-width = 2
keyword-case = "upper"    # "upper" | "lower"
semicolons = true
```

## Schemas

The `[schemas]` section maps glob patterns to schema DDL files. Globs are
matched against the SQL file's path relative to the directory containing
`syntaqlite.toml`.

Resolution order (first match wins):

1. `[schemas]` glob entries — checked in file order, first match wins
2. `schema` top-level key — fallback for files that don't match any glob
3. No schema — syntax-only validation (no table/column checks)

Schema file paths are relative to the directory containing `syntaqlite.toml`.

### Why glob-based routing

Real projects have multiple schemas. A web app might have the main application
schema, a separate analytics schema, and test fixture tables. Migration files
shouldn't be validated against any schema because they *define* the schema.
Glob routing handles all of these:

```toml
[schemas]
"src/analytics/**/*.sql" = ["schema/analytics.sql"]
"src/**/*.sql" = ["schema/app.sql"]
"tests/**/*.sql" = ["schema/app.sql", "schema/test_fixtures.sql"]
"migrations/**/*.sql" = []
```

### Schema files

Schema files are SQL DDL — the same format you'd get from `sqlite3 mydb.db .schema`:

```sql
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT UNIQUE);
CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER REFERENCES users(id), amount REAL);
```

You can split your schema across multiple files and reference them all in the
config:

```toml
[schemas]
"**/*.sql" = ["schema/tables.sql", "schema/views.sql", "schema/functions.sql"]
```

## Format options

The `[format]` section sets default formatting options. All fields are optional —
omitted fields use the built-in defaults.

| Key | Default | Description |
|-----|---------|-------------|
| `line-width` | `80` | Target maximum line width |
| `indent-width` | `2` | Spaces per indentation level |
| `keyword-case` | `"upper"` | `"upper"` or `"lower"` |
| `semicolons` | `true` | Append semicolons after statements |

See [Formatting options](@/reference/formatting-options.md) for detailed
descriptions and examples of each option.

## CLI override

CLI flags always override config file values. This lets you use the config file
as the project default while still overriding on a per-invocation basis:

```bash
# Uses config file defaults
syntaqlite fmt query.sql

# CLI flag overrides config file
syntaqlite fmt -w 120 query.sql
```

The same applies to `--schema` on `syntaqlite validate`:

```bash
# Uses config file schemas
syntaqlite validate src/query.sql

# CLI flag overrides config file
syntaqlite validate --schema other.sql src/query.sql
```

## Where config is read

| Consumer | How it works |
|----------|--------------|
| `syntaqlite fmt` | Discovers config from input file directory or cwd. Format options from config are defaults; CLI flags override. |
| `syntaqlite validate` | Discovers config from input file directory or cwd. Schema resolution from config when `--schema` is not given. |
| `syntaqlite lsp` | Discovers config from cwd at startup. Loads schema catalog and format config for all LSP operations. |
| VS Code extension | No configuration needed — the LSP reads `syntaqlite.toml` directly. |
| Claude Code plugin | No configuration needed — the LSP reads `syntaqlite.toml` directly. |
| Neovim, Helix, etc. | No configuration needed — the LSP reads `syntaqlite.toml` directly. |

## Minimal example

For a project with a single schema file:

```toml
schema = ["schema.sql"]
```

This validates all SQL files against `schema.sql` and uses default formatting.

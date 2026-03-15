+++
title = "Config file"
description = "syntaqlite.toml format — schemas, formatting, and discovery."
weight = 3
+++

# Config file

Project configuration lives in `syntaqlite.toml`. The CLI, LSP server, and all
editor integrations read it.

## Discovery

syntaqlite walks up from the file being processed (or the current directory for
CLI commands) and uses the first `syntaqlite.toml` it finds.

## File format

```toml
# Default schema for SQL files that don't match any glob in [schemas].
# Optional — if omitted, unmatched files get no schema.
# schema = ["schema.sql"]

# Schema DDL files for validation and completions.
# Each entry maps a glob pattern to schema file(s).
[schemas]
"src/**/*.sql" = ["schema/main.sql", "schema/views.sql"]
"tests/**/*.sql" = ["schema/main.sql", "schema/test_fixtures.sql"]
"migrations/*.sql" = []  # no schema validation

# Formatting options (all optional, shown with defaults).
[format]
line-width = 80
indent-width = 2
keyword-case = "upper"    # "upper" | "lower"
semicolons = true
```

## `schema`

Top-level key. Default schemas applied to SQL files that don't match any
`[schemas]` glob.

- Type: array of strings (file paths relative to the config file directory)
- Optional

## `[schemas]`

Maps glob patterns to schema DDL file(s). Globs are matched against the SQL
file's path relative to the directory containing `syntaqlite.toml`. Entries are
checked in file order; first match wins.

- Type: table of `"glob pattern" = ["schema_file.sql", ...]`
- Optional

Schema resolution order:

1. `[schemas]` glob entries — first matching glob wins
2. `schema` top-level key — fallback for unmatched files
3. No schema — syntax-only checks

Schema files contain SQL DDL (`CREATE TABLE`, `CREATE VIEW`, etc.) — the same
format as `sqlite3 mydb.db .schema` output.

## `[format]`

Default formatting options. All fields are optional — omitted fields use
built-in defaults. CLI flags override these values.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `line-width` | integer | `80` | Target maximum line width |
| `indent-width` | integer | `2` | Spaces per indentation level |
| `keyword-case` | string | `"upper"` | `"upper"` or `"lower"` |
| `semicolons` | boolean | `true` | Append semicolons after statements |

See [Formatting options](@/reference/formatting-options.md) for detailed
descriptions of each option.

## Precedence

CLI flags always override config file values. When `--schema` is passed to
`syntaqlite validate`, it takes precedence over `[schemas]` and `schema`.

## Consumers

| Consumer | Behavior |
|----------|----------|
| `syntaqlite fmt` | Discovers config from input file directory or cwd. Format options are defaults; CLI flags override. |
| `syntaqlite validate` | Discovers config from input file directory or cwd. Schema resolution from config when `--schema` is not given. |
| `syntaqlite lsp` | Discovers config from cwd at startup. Loads schema catalog and format config. |
| VS Code, Claude Code, Neovim, Helix | The LSP reads `syntaqlite.toml` directly — no editor-specific configuration needed. |

+++
title = "Project setup"
description = "Configure syntaqlite.toml for a real project — schema, formatting, version pinning, and checks."
weight = 0
+++

# Project setup

A `syntaqlite.toml` in your project root is the single source of configuration
for the entire syntaqlite toolchain. Every tool — the CLI, the VS Code
extension, Neovim/Helix via the LSP, Claude Code via MCP, and CI — discovers
and reads this file automatically. Configure once, and the whole team gets
consistent formatting, validation, and diagnostics everywhere.

## Schema

Point syntaqlite at your DDL so it can validate table, column, and function
references. The simplest setup uses the top-level `schema` key:

```toml
schema = ["schema.sql"]
```

Schema files use the same format as `sqlite3 mydb.db .schema` output — you
can export directly from an existing database.

For projects where different directories use different schemas, use glob-based
routing. First match wins; unmatched files fall back to the top-level `schema`
key:

```toml
schema = ["schema/main.sql"]

[schemas]
"src/**/*.sql"   = ["schema/main.sql", "schema/views.sql"]
"tests/**/*.sql" = ["schema/main.sql", "schema/test_fixtures.sql"]
"migrations/*.sql" = []
```

Setting a glob to `[]` disables schema validation for those files.

## SQLite version and compile flags

Pin the version and compile-time flags to match your production environment.
This prevents syntaqlite from accepting syntax or functions that won't exist
at runtime:

```toml
sqlite-version = "3.41.0"
sqlite-cflags = [
  "SQLITE_ENABLE_MATH_FUNCTIONS",
  "SQLITE_ENABLE_FTS5",
]
```

Without `sqlite-version`, syntaqlite defaults to `latest`. Without
`sqlite-cflags`, no optional features are enabled — functions like `sin()` or
`fts5()` will be flagged as unknown.

See [SQLite version and compile flags](@/guides/sqlite-versions.md) for the
full list of supported flags.

## Formatting

```toml
[format]
line-width = 100
indent-width = 4
keyword-case = "upper"
semicolons = true
```

These are defaults — CLI flags like `--line-width` override them for one-off
use. All values are optional; omitted fields use built-in defaults.

## Check levels

When a schema is provided, schema checks (`unknown-table`, `unknown-column`,
etc.) default to `"deny"`. Override per category with the `[checks]` section:

```toml
[checks]
schema = "deny"              # shorthand for all schema checks
unknown-function = "warn"    # per-category override
function-arity = "allow"
```

Each category accepts `"allow"`, `"warn"`, or `"deny"`. The `schema` and `all`
shorthands set multiple categories at once; per-category keys override them.

## Putting it together

A typical production config:

```toml
sqlite-version = "3.41.0"
sqlite-cflags = ["SQLITE_ENABLE_MATH_FUNCTIONS"]

[schemas]
"src/**/*.sql"   = ["schema/main.sql"]
"tests/**/*.sql" = ["schema/main.sql", "schema/test_fixtures.sql"]

[format]
line-width = 100
indent-width = 4

[checks]
schema = "deny"
```

Once committed, this config applies everywhere — `syntaqlite fmt` and
`syntaqlite validate` on the command line, the VS Code extension, Neovim and
Helix via the language server, Claude Code via MCP, and CI pipelines. No
per-tool or per-editor configuration needed.

## Precedence

CLI flags always override config file values. This lets you use the config for
team-wide defaults while still allowing local overrides:

```bash
# Uses config defaults
syntaqlite validate "**/*.sql"

# Overrides just the version for this run
syntaqlite validate --sqlite-version 3.46.0 "**/*.sql"
```

## Next steps

- [Config file reference](@/reference/config-file.md) — full format
  specification
- [Schema validation](@/guides/schema-validation.md) — how schema resolution
  works in detail
- [Formatting in CI](@/guides/ci-integration.md) — GitHub Actions and pre-push
  hooks

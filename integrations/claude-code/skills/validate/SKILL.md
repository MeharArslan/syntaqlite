---
name: validate
description: Validate SQL and report diagnostics using syntaqlite. Use when the user wants to check SQL for errors, lint SQL files, or verify correctness against a schema.
---

# Validate SQL

Validate SQLite SQL and report diagnostics (errors, warnings) using the syntaqlite CLI.

## Usage

```bash
# Validate a file
syntaqlite validate query.sql

# Validate from stdin
echo "SELECT * FORM t" | syntaqlite validate

# Validate an inline expression
syntaqlite validate -e "SELECT 1 + "

# Validate against a schema
syntaqlite validate --schema schema.sql query.sql

# Validate multiple files via glob
syntaqlite validate "**/*.sql"

# Validate with multiple schema files
syntaqlite validate --schema "migrations/*.sql" --schema extra.sql query.sql
```

## Options

- `-e, --expression <SQL>` — Validate an inline SQL expression instead of files
- `--schema <FILE>` — Schema DDL file to load before validation (repeatable, supports globs)
- `--experimental-lang <LANG>` — Extract embedded SQL from host language files (`python`, `typescript`)

## Notes

- Without `--schema`, validation checks syntax only. With `--schema`, it also checks table/column references.
- Exit code is 0 if valid, non-zero if errors are found.
- For continuous validation while editing, the LSP server provides real-time diagnostics automatically.

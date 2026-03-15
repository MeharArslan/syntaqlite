+++
title = "Command line"
description = "Install the CLI for formatting, validation, CI, and scripting."
weight = 3
+++

# Command line

## Install

**Homebrew (macOS):**

```bash
brew install LalitMaganti/tap/syntaqlite
```

**pip (all platforms):**

```bash
pip install syntaqlite
```

**Cargo:**

```bash
cargo install syntaqlite-cli
```

**Download binary (all platforms):**

```bash
curl -sSf https://raw.githubusercontent.com/LalitMaganti/syntaqlite/main/tools/syntaqlite | python3 - fmt -e "select 1"
```

Downloads the binary to `~/.local/share/syntaqlite/` on first run, then executes it. Subsequent runs use the cached binary.

Verify it works:

```bash
syntaqlite --help
```

## Format SQL

```bash
# Inline expression
syntaqlite fmt -e "select a,b,c from users where id=1 and active=true"

# From stdin
echo "select a,b,c from users where id=1 and active=true" | syntaqlite fmt
```

Output:

```sql
SELECT a, b, c
FROM users
WHERE id = 1
  AND active = true;
```

Format a file in place:

```bash
syntaqlite fmt -i query.sql
```

Format all SQL files in a project:

```bash
syntaqlite fmt -i "**/*.sql"
```

Options:

| Flag | Default | Description |
|------|---------|-------------|
| `-w, --line-width` | 80 | Target maximum line width |
| `-k, --keyword-case` | `upper` | `upper` or `lower` |
| `--semicolons` | `true` | Append semicolons after statements |
| `-i, --in-place` | | Write output back to files |

## Validate SQL

Separate your schema (DDL) from the queries you want to check:

```bash
syntaqlite validate --schema schema.sql queries.sql
```

Multiple schema files are supported — use `--schema` more than once or pass a
glob:

```bash
syntaqlite validate --schema "db/*.sql" queries.sql
```

Example schema file (`schema.sql`):

```sql
CREATE TABLE users (id INTEGER, name TEXT, email TEXT);
```

Example query file (`queries.sql`):

```sql
SELECT nme FROM users;
```

```text
error: unknown column 'nme'
 --> queries.sql:1:8
  |
1 | SELECT nme FROM users;
  |        ^^^
  |
  = help: did you mean 'name'?
```

You can also put DDL and queries in the same file (without `--schema`) for quick
one-off checks:

```bash
echo "CREATE TABLE t (a INT); SELECT b FROM t;" | syntaqlite validate
```

### Embedded SQL (experimental)

Validate SQL strings extracted from Python or TypeScript source files:

```bash
syntaqlite validate --experimental-lang python app.py
syntaqlite validate --experimental-lang typescript app.ts
```

### SQLite version pinning

Match your production SQLite version and compile flags:

```bash
syntaqlite validate --sqlite-version 3.41.0 \
  --sqlite-cflag SQLITE_ENABLE_MATH_FUNCTIONS \
  query.sql
```

See [SQLite version and compile flags](@/guides/sqlite-versions.md) for
details.

## Inspect the parse tree

```bash
echo "SELECT 1 + 2" | syntaqlite parse -o text
```

Prints a text dump of the abstract syntax tree.

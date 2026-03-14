+++
title = "Command line"
description = "Install the CLI for formatting, validation, CI, and scripting."
weight = 3
+++

# Command line

## Install

**macOS / Linux (Homebrew):**

```bash
brew install LalitMaganti/tap/syntaqlite
```

**macOS / Linux (shell installer):**

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/LalitMaganti/syntaqlite/releases/latest/download/syntaqlite-cli-installer.sh \
  | sh
```

**Windows:**

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/LalitMaganti/syntaqlite/releases/latest/download/syntaqlite-cli-installer.ps1 | iex"
```

**Cargo:**

```bash
cargo install syntaqlite-cli
```

Verify it works:

```bash
syntaqlite --help
```

## Format SQL

```bash
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

```bash
syntaqlite validate schema.sql
```

If the file contains `CREATE TABLE` statements followed by queries, syntaqlite
builds a schema from the DDL and validates the queries against it:

```sql
CREATE TABLE users (id INTEGER, name TEXT, email TEXT);

SELECT nme FROM users;
```

```text
error: unknown column 'nme'
 --> schema.sql:3:8
  |
3 | SELECT nme FROM users;
  |        ^^^
  |
  = help: did you mean 'name'?
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
echo "SELECT 1 + 2" | syntaqlite parse -o ast
```

Prints a text dump of the abstract syntax tree.

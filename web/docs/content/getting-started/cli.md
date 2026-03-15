+++
title = "Command line"
description = "Install the CLI for formatting, validation, CI, and scripting."
weight = 3
+++

# Command line

## Install

<div class="tabs" data-tab-group="cli-install">
  <div class="tab-buttons">
    <button class="active" data-tab="binary" onclick="switchTab('cli-install','binary')">Download script</button>
    <button data-tab="mise" onclick="switchTab('cli-install','mise')">mise</button>
    <button data-tab="pip" onclick="switchTab('cli-install','pip')">pip</button>
    <button data-tab="brew" onclick="switchTab('cli-install','brew')">Homebrew</button>
    <button data-tab="cargo" onclick="switchTab('cli-install','cargo')">Cargo</button>
  </div>
  <div class="tab-panel active" data-tab="binary">
    <pre><code class="language-bash">curl -sSf https://raw.githubusercontent.com/LalitMaganti/syntaqlite/main/tools/syntaqlite | python3 - install</code></pre>
    <p>Downloads the latest release to <code>~/.local/bin</code>. Works on macOS, Linux, and Windows. Optionally pass a custom directory: <code>python3 - install /usr/local/bin</code>.</p>
  </div>
  <div class="tab-panel" data-tab="mise">
    <pre><code class="language-bash">mise use github:LalitMaganti/syntaqlite</code></pre>
  </div>
  <div class="tab-panel" data-tab="pip">
    <pre><code class="language-bash">pip install syntaqlite</code></pre>
    <p>Installs a bundled platform-specific binary — no Rust toolchain needed.</p>
  </div>
  <div class="tab-panel" data-tab="brew">
    <pre><code class="language-bash">brew install LalitMaganti/tap/syntaqlite</code></pre>
  </div>
  <div class="tab-panel" data-tab="cargo">
    <pre><code class="language-bash">cargo install syntaqlite-cli</code></pre>
  </div>
</div>

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
| `--check` | | Check formatting without modifying (exit 1 if unformatted) |

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
echo "SELECT 1 + 2" | syntaqlite parse
```

Prints a text dump of the abstract syntax tree. See
[parsing guide](@/guides/parsing.md) for details.

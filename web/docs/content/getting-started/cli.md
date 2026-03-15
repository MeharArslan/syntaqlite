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
syntaqlite fmt -e "select a,b,c from users where id=1 and active=true"
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

See [CLI reference](@/reference/cli.md) for all formatting flags and
[`syntaqlite.toml`](@/reference/config-file.md) for project-wide defaults.

## Validate SQL

Create a schema file (`schema.sql`):

```sql
CREATE TABLE users (id INTEGER, name TEXT, email TEXT);
```

Validate a query against it:

```bash
syntaqlite validate --schema schema.sql -e "SELECT nme FROM users"
```

```text
error: unknown column 'nme'
 --> <expression>:1:8
  |
1 | SELECT nme FROM users
  |        ^^^
  |
  = help: did you mean 'name'?
```

You can also put DDL and queries in the same file for quick one-off checks:

```bash
echo "CREATE TABLE t (a INT); SELECT b FROM t;" | syntaqlite validate
```

See [CLI reference](@/reference/cli.md) for all validation flags and
[`syntaqlite.toml`](@/reference/config-file.md) for configuring schemas
per project.

## Inspect the parse tree

```bash
echo "SELECT 1 + 2" | syntaqlite parse
```

Prints a text dump of the abstract syntax tree. See
[parsing guide](@/guides/parsing.md) for details.

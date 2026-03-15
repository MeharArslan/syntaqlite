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

Check that it installed:

```bash
syntaqlite --version
```

## Format a query

Run this to see formatting in action:

```bash
syntaqlite fmt -e "select id,name,email from users where active=1 and role='admin' order by name"
```

Output:

```sql
SELECT id, name, email
FROM users
WHERE active = 1
  AND role = 'admin'
ORDER BY name;
```

Keywords are uppercased, clauses break onto separate lines, and a semicolon is
appended.

## Format a file

Create a file called `query.sql`:

```sql
select u.id,u.name,p.title from users u join posts p on u.id=p.user_id where u.active=1
```

Format it in place:

```bash
syntaqlite fmt -i query.sql
cat query.sql
```

```sql
SELECT u.id, u.name, p.title
FROM users u
  JOIN posts p ON u.id = p.user_id
WHERE u.active = 1;
```

To format every SQL file in a project at once:

```bash
syntaqlite fmt -i "**/*.sql"
```

## Validate against a schema

Create a schema file (`schema.sql`) with your table definitions:

```sql
CREATE TABLE users (id INTEGER, name TEXT, email TEXT, active INTEGER);
```

Now validate a query that has a typo:

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

syntaqlite found the typo and suggested the correct column name. For quick
one-off checks, you can put DDL and queries in the same file without
`--schema`:

```bash
echo "CREATE TABLE t (a INT); SELECT b FROM t;" | syntaqlite validate
```

```text
warning: unknown column 'b'
 --> <stdin>:1:33
  |
1 | CREATE TABLE t (a INT); SELECT b FROM t;
  |                                 ^
  |
  = help: did you mean 'a'?
```

## Set up project config

Instead of passing `--schema` every time, create a `syntaqlite.toml` in your
project root:

```toml
schema = ["schema.sql"]
```

Now `syntaqlite validate query.sql` picks up the schema automatically. The
config file also sets formatting defaults — see the
[config file reference](@/reference/config-file.md) for the full format.

## Check formatting in CI

Use `--check` to verify files are formatted without modifying them:

```bash
syntaqlite fmt --check "**/*.sql"
```

This exits with code 1 if any file would change — useful in CI pipelines and
pre-push hooks.

## Next steps

- [CLI reference](@/reference/cli.md) — all flags for `fmt`, `validate`,
  `parse`, and `lsp`
- [Config file reference](@/reference/config-file.md) — `syntaqlite.toml`
  format
- [Formatting options](@/reference/formatting-options.md) — line width,
  keyword casing, and more

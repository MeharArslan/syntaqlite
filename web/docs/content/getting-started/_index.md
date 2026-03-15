+++
title = "Getting started"
sort_by = "weight"
+++

# Getting started

syntaqlite is a parser, formatter, validator, and language server for SQLite SQL.
You can [try it in the browser](https://playground.syntaqlite.com) without
installing anything, or install the CLI:

<div class="tabs" data-tab-group="install">
  <div class="tab-buttons">
    <button class="active" data-tab="binary" onclick="switchTab('install','binary')">Download &amp; run</button>
    <button data-tab="mise" onclick="switchTab('install','mise')">mise</button>
    <button data-tab="pip" onclick="switchTab('install','pip')">pip</button>
    <button data-tab="brew" onclick="switchTab('install','brew')">Homebrew</button>
    <button data-tab="cargo" onclick="switchTab('install','cargo')">Cargo</button>
  </div>
  <div class="tab-panel active" data-tab="binary">
    <pre><code class="language-bash">curl -sSf https://raw.githubusercontent.com/LalitMaganti/syntaqlite/main/tools/syntaqlite | python3 - fmt -e "select 1"</code></pre>
    <p>Downloads the binary on first run, caches it, auto-updates weekly. Works on macOS, Linux, and Windows.</p>
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

## Format

```bash
syntaqlite fmt -e "select id,name from users where active=1 and role='admin'"
```
```sql
SELECT id, name FROM users WHERE active = 1 AND role = 'admin';
```

Format a file in place with `syntaqlite fmt -i query.sql`, or check formatting
in CI with `syntaqlite fmt --check "**/*.sql"`.

## Validate

syntaqlite reads your `CREATE TABLE` statements to build a schema, then
validates queries against it — no database required. It finds **all** errors in
one pass, with source locations and did-you-mean suggestions:

```bash
syntaqlite validate -e "CREATE TABLE users (id, name, email); SELECT nme FROM users;"
```
```text
warning: unknown column 'nme'
 --> <expression>:1:46
  |
1 | CREATE TABLE users (id, name, email); SELECT nme FROM users;
  |                                              ^~~
  = help: did you mean 'name'?
```

For real projects, separate your schema from your queries:

```bash
syntaqlite validate --schema schema.sql queries.sql
```

### Version pinning

If you're targeting a specific SQLite version (e.g. Android 13 ships SQLite
3.32.2), syntaqlite catches syntax that wouldn't exist yet:

```bash
syntaqlite --sqlite-version 3.32.0 validate query.sql
```

## Parse

Inspect the full abstract syntax tree:

```bash
syntaqlite parse -e "SELECT 1 + 2" --output text
```

Useful for code generation, migration tooling, or static analysis. See
[parsing guide](@/guides/parsing.md) for details.

## What's next

<div class="entry-cards">
  <a href="{{ get_url(path='getting-started/cli') }}" class="entry-card">
    <span class="entry-card__title">CLI reference</span>
    <span class="entry-card__desc">All commands, flags, and options.</span>
  </a>
  <a href="{{ get_url(path='getting-started/vscode') }}" class="entry-card">
    <span class="entry-card__title">VS Code</span>
    <span class="entry-card__desc">Diagnostics, formatting, and completions in your editor.</span>
  </a>
  <a href="{{ get_url(path='getting-started/claude-code') }}" class="entry-card">
    <span class="entry-card__title">Claude Code / MCP</span>
    <span class="entry-card__desc">Plugin and MCP server for AI coding assistants.</span>
  </a>
  <a href="{{ get_url(path='getting-started/other-editors') }}" class="entry-card">
    <span class="entry-card__title">Other editors</span>
    <span class="entry-card__desc">Neovim, Helix, or any editor with LSP support.</span>
  </a>
  <a href="{{ get_url(path='getting-started/wasm-js') }}" class="entry-card">
    <span class="entry-card__title">WASM / JavaScript</span>
    <span class="entry-card__desc">Use syntaqlite in the browser or Node.js.</span>
  </a>
</div>

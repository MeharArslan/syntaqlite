+++
title = "Getting started"
sort_by = "weight"
+++

# Getting started

Choose how you want to use syntaqlite, then follow the guide for your setup.

## Quick taste

Install the CLI (or [try it in the browser](https://playground.syntaqlite.com) first):

<div class="tabs" data-tab-group="install">
  <div class="tab-buttons">
    <button class="active" data-tab="brew" onclick="switchTab('install','brew')">Homebrew</button>
    <button data-tab="shell" onclick="switchTab('install','shell')">Shell</button>
    <button data-tab="windows" onclick="switchTab('install','windows')">Windows</button>
    <button data-tab="cargo" onclick="switchTab('install','cargo')">Cargo</button>
  </div>
  <div class="tab-panel active" data-tab="brew">
    <pre><code class="language-bash">brew install LalitMaganti/tap/syntaqlite</code></pre>
  </div>
  <div class="tab-panel" data-tab="shell">
    <pre><code class="language-bash">curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/LalitMaganti/syntaqlite/releases/latest/download/syntaqlite-cli-installer.sh \
  | sh</code></pre>
  </div>
  <div class="tab-panel" data-tab="windows">
    <pre><code class="language-powershell">powershell -ExecutionPolicy ByPass -c "irm https://github.com/LalitMaganti/syntaqlite/releases/latest/download/syntaqlite-cli-installer.ps1 | iex"</code></pre>
  </div>
  <div class="tab-panel" data-tab="cargo">
    <pre><code class="language-bash">cargo install syntaqlite-cli</code></pre>
  </div>
</div>

Format a query:

```bash
echo "select id,name from users where active=1 and role='admin'" | syntaqlite fmt
```
```sql
SELECT id, name
FROM users
WHERE active = 1
  AND role = 'admin';
```

Catch a schema error:

```bash
echo "CREATE TABLE users (id, name, email); SELECT nme FROM users;" | syntaqlite validate
```
```text
error: unknown column 'nme'
 --> stdin:1:43
  |
1 | ...; SELECT nme FROM users;
  |             ^^^
  |
  = help: did you mean 'name'?
```

That's the core loop: syntaqlite reads your `CREATE TABLE` statements to build
a schema, then validates queries against it — no database required.

## Choose your setup

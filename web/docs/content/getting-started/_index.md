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
    <button class="active" data-tab="binary" onclick="switchTab('install','binary')">Download &amp; run</button>
    <button data-tab="mise" onclick="switchTab('install','mise')">mise</button>
    <button data-tab="pip" onclick="switchTab('install','pip')">pip</button>
    <button data-tab="brew" onclick="switchTab('install','brew')">Homebrew</button>
    <button data-tab="cargo" onclick="switchTab('install','cargo')">Cargo</button>
  </div>
  <div class="tab-panel active" data-tab="binary">
    <pre><code class="language-bash">curl -sSf https://raw.githubusercontent.com/LalitMaganti/syntaqlite/main/tools/syntaqlite | python3 - fmt -e "select 1"</code></pre>
  </div>
  <div class="tab-panel" data-tab="mise">
    <pre><code class="language-bash">mise use ubi:LalitMaganti/syntaqlite</code></pre>
  </div>
  <div class="tab-panel" data-tab="pip">
    <pre><code class="language-bash">pip install syntaqlite</code></pre>
  </div>
  <div class="tab-panel" data-tab="brew">
    <pre><code class="language-bash">brew install LalitMaganti/tap/syntaqlite</code></pre>
  </div>
  <div class="tab-panel" data-tab="cargo">
    <pre><code class="language-bash">cargo install syntaqlite-cli</code></pre>
  </div>
</div>

Format a query:

```bash
syntaqlite fmt -e "select id,name from users where active=1 and role='admin'"
```
```sql
SELECT id, name FROM users WHERE active = 1 AND role = 'admin';
```

Catch a schema error:

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

That's the core loop: syntaqlite reads your `CREATE TABLE` statements to build
a schema, then validates queries against it — no database required.

## Choose your setup

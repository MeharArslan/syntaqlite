+++
title = "Validating SQL"
description = "Run semantic analysis to catch unknown tables, columns, and functions."
weight = 3
+++

# Validating SQL

syntaqlite's validator goes beyond syntax checking — it builds a schema from
`CREATE TABLE` statements and checks that queries reference real tables,
columns, and functions. This page covers practical validation workflows.

## CLI basics

Pass SQL files (or glob patterns) to `syntaqlite validate`:

```bash
syntaqlite validate "schema/**/*.sql" "queries/**/*.sql"
```

File order matters: put your DDL files first so the schema is available when
queries are validated. Within each file, `CREATE TABLE` and `CREATE VIEW`
statements are processed in order and made available to subsequent statements.

If no files are given, syntaqlite reads from stdin:

```bash
cat schema.sql queries.sql | syntaqlite validate
```

## How schema is built

The validator recognizes DDL statements and extracts schema information from
them:

```sql
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT);
CREATE TABLE posts (id INTEGER, user_id INTEGER, title TEXT, body TEXT);

-- This query is validated against the schema above:
SELECT u.name, p.title
FROM users u
JOIN posts p ON p.user_id = u.id
WHERE u.emal = 'alice@example.com';
```

```text
warning: unknown column 'emal' in table 'u'
 --> <stdin>:7:9
  |
7 | WHERE u.emal = 'alice@example.com';
  |         ^~~~
  = help: did you mean 'email'?
```

The validator checks:

- **Unknown tables** — `FROM`, `JOIN`, `INSERT INTO`, `UPDATE`, `DELETE FROM`
  references
- **Unknown columns** — column references qualified or unqualified, checked
  against the table's column list
- **Unknown functions** — function calls checked against SQLite's built-in
  function catalog
- **Function arity** — wrong number of arguments to known functions
- **CTE column count** — mismatch between declared CTE column list and the
  number of columns the CTE's `SELECT` produces

### "Did you mean?" suggestions

When a name doesn't match, the validator uses Levenshtein distance to suggest
corrections. This works for table names, column names, and function names.

## Embedded SQL

Validate SQL strings embedded in Python or TypeScript source files:

```bash
syntaqlite validate --experimental-lang python app.py
syntaqlite validate --experimental-lang typescript db.ts
```

syntaqlite extracts SQL string literals from the host language, then runs
validation on each fragment. This is experimental — complex string
interpolation patterns may not be recognized.

## SQLite version pinning

Match your production SQLite version and compile-time flags:

```bash
syntaqlite validate \
  --sqlite-version 3.41.0 \
  --sqlite-cflag SQLITE_ENABLE_MATH_FUNCTIONS \
  query.sql
```

This affects which built-in functions are recognized. See
[SQLite version and compile flags](@/guides/sqlite-versions.md) for the full
list of flags.

## Using validation from Rust

Add syntaqlite with the `validation` feature:

```toml
[dependencies]
syntaqlite = { version = "0.0.14", features = ["validation", "sqlite"] }
```

The main types are
[`SemanticAnalyzer`](https://github.com/LalitMaganti/syntaqlite/blob/main/syntaqlite/src/semantic/analyzer.rs),
[`Catalog`](https://github.com/LalitMaganti/syntaqlite/blob/main/syntaqlite/src/semantic/catalog.rs),
and
[`ValidationConfig`](https://github.com/LalitMaganti/syntaqlite/blob/main/syntaqlite/src/semantic/mod.rs):

```rust
use syntaqlite::semantic::{
    SemanticAnalyzer, Catalog, CatalogLayer, ValidationConfig,
};
use syntaqlite::sqlite_dialect;

// 1. Create a reusable analyzer
let mut analyzer = SemanticAnalyzer::new();

// 2. Define your schema
let mut catalog = Catalog::new(sqlite_dialect());
catalog.layer_mut(CatalogLayer::Database)
    .insert_table("users", Some(vec!["id".into(), "name".into(), "email".into()]), false);
catalog.layer_mut(CatalogLayer::Database)
    .insert_table("posts", Some(vec!["id".into(), "user_id".into(), "title".into()]), false);

// 3. Run analysis
let config = ValidationConfig::default();
let model = analyzer.analyze(
    "SELECT nme FROM users",
    &catalog,
    &config,
);

// 4. Inspect diagnostics
for diag in model.diagnostics() {
    println!("[{}] {}", diag.severity(), diag.message());
    if let Some(help) = diag.help() {
        println!("  help: {help}");
    }
}
```

### Catalog layers

The catalog has a layered resolution order (innermost wins):

| Layer | Purpose |
|-------|---------|
| Query | CTEs, subquery aliases (managed automatically) |
| Document | `CREATE TABLE` statements in the current file |
| Connection | DDL accumulated across calls (Execute mode) |
| Database | User-provided schema (the layer you populate) |
| Dialect | Built-in functions |

For most use cases, populate the `Database` layer with your schema and let the
analyzer handle the rest.

### Tables with unknown columns

If you know a table exists but don't know its columns, pass `None`:

```rust
catalog.layer_mut(CatalogLayer::Database)
    .insert_table("legacy_table", None, false);
```

This suppresses unknown-column warnings for that table — any column reference
will be accepted.

### Strict mode

By default, unresolved names produce warnings. To make them errors:

```rust
let config = ValidationConfig::default()
    .with_strict_schema(true);
```

### Analysis modes

The analyzer supports two modes (set via `SemanticAnalyzer::with_mode()`):

- **Document** (default) — DDL resets between `analyze()` calls. Use this for
  file-at-a-time validation (editor, CI).
- **Execute** — DDL accumulates across calls. Use this for interactive
  sessions (REPL-style).

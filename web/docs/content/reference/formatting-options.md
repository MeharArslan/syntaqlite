+++
title = "Formatting options"
description = "All formatting configuration options with examples."
weight = 2
+++

# Formatting options

syntaqlite's formatter has four configuration options. All have sensible
defaults — you can use `syntaqlite fmt` with no flags and get well-formatted
SQL. Every option is available across the CLI, Rust, C, and JavaScript APIs.

For project-wide defaults, set options in
[`syntaqlite.toml`](@/reference/config-file.md) so every team member and CI
job uses the same settings without passing flags:

## Options

| Option | CLI flag | Config file key | Default | Description |
|--------|----------|-----------------|---------|-------------|
| Line width | `-w, --line-width <N>` | `line-width` | `80` | Target maximum line width (characters) |
| Indent width | `-t, --indent-width <N>` | `indent-width` | `2` | Spaces per indentation level |
| Keyword case | `-k, --keyword-case <CASE>` | `keyword-case` | `upper` | `upper` or `lower` |
| Semicolons | `--semicolons <BOOL>` | `semicolons` | `true` | Append `;` after each statement |

## Line width

Controls when the formatter breaks a statement across multiple lines. The
formatter tries to fit as much as possible within this width, breaking at
natural points (before clauses, between list items) when the line would be too
long.

```bash
# Narrow: breaks more aggressively
echo "SELECT id, name, email, created_at FROM users WHERE active = 1" \
  | syntaqlite fmt -w 40
```

```sql
SELECT id, name, email, created_at
FROM users
WHERE
  active = 1;
```

```bash
# Wide: keeps more on one line
echo "SELECT id, name, email, created_at FROM users WHERE active = 1" \
  | syntaqlite fmt -w 120
```

```sql
SELECT id, name, email, created_at FROM users WHERE active = 1;
```

The line width is a target, not a hard limit — the formatter won't break a
single long identifier or string literal to stay within the width.

## Indent width

The number of spaces used for each indentation level (e.g., continuation of
`WHERE` conditions, subqueries, column lists that break across lines).

```bash
echo "SELECT id, name FROM users WHERE active = 1 AND role = 'admin'" \
  | syntaqlite fmt -t 4
```

## Keyword case

Controls whether SQL keywords are uppercased or lowercased. Identifiers,
string literals, and other non-keyword tokens are never modified.

```bash
echo "Select Id From Users Where Active = true" | syntaqlite fmt -k upper
```

```sql
SELECT Id FROM Users WHERE Active = true;
```

```bash
echo "Select Id From Users Where Active = true" | syntaqlite fmt -k lower
```

```sql
select Id from Users where Active = true;
```

## Semicolons

Controls whether a semicolon is appended after each statement.

```bash
echo "SELECT 1" | syntaqlite fmt --semicolons=true
```

```sql
SELECT 1;
```

```bash
echo "SELECT 1" | syntaqlite fmt --semicolons=false
```

```sql
SELECT 1
```

## Rust API

All options are set via the builder pattern on
[`FormatConfig`](https://docs.rs/syntaqlite/latest/syntaqlite/fmt/struct.FormatConfig.html):

```rust
use syntaqlite::{Formatter, FormatConfig, KeywordCase};

let config = FormatConfig::default()
    .with_line_width(120)
    .with_indent_width(4)
    .with_keyword_case(KeywordCase::Lower)
    .with_semicolons(false);

let mut fmt = Formatter::with_config(&config);
let output = fmt.format("SELECT 1")?;
```

See the [Rust API reference](@/reference/rust-api.md) for all types and
methods.

## C API

The C FFI exposes the same options via `SyntaqliteFormatConfig`:

```c
SyntaqliteFormatConfig config = {
    .line_width   = 120,
    .indent_width = 4,
    .keyword_case = SYNTAQLITE_KEYWORD_LOWER,
    .semicolons   = 0,
};
SyntaqliteFormatter* f =
    syntaqlite_formatter_create_sqlite_with_config(&config);
```

See the [C API reference](@/reference/c-api.md) for all functions and the
memory model.

## JavaScript (WASM)

```typescript
import {FormatOptions} from "syntaqlite";

const opts: FormatOptions = {
  lineWidth: 120,
  indentWidth: 4,
  keywordCase: 2, // 0 = preserve, 1 = upper, 2 = lower
  semicolons: false,
};
const result = runtime.runFmt(sql, opts);
```

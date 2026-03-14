+++
title = "Formatting options"
description = "All formatting configuration options with examples."
weight = 2
+++

# Formatting options

syntaqlite's formatter has four configuration options. All have sensible
defaults — you can use `syntaqlite fmt` with no flags and get well-formatted
SQL.

## Options

| Option | CLI flag | Default | Description |
|--------|----------|---------|-------------|
| Line width | `-w, --line-width <N>` | `80` | Target maximum line width (characters) |
| Keyword case | `-k, --keyword-case <CASE>` | `upper` | `upper` or `lower` |
| Semicolons | `--semicolons <BOOL>` | `true` | Append `;` after each statement |
| Indent width | *(Rust API only)* | `2` | Spaces per indentation level |

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

## Indent width

The number of spaces used for each indentation level (e.g., continuation of
`WHERE` conditions, subqueries, column lists that break across lines).

This option is available through the Rust API but not exposed as a CLI flag:

```rust
use syntaqlite::{Formatter, FormatConfig};

let config = FormatConfig::default().with_indent_width(4);
let mut fmt = Formatter::with_config(&config);
let output = fmt.format("SELECT id, name FROM users WHERE active = 1 AND role = 'admin'")?;
```

## Rust API

All options are set via the builder pattern on
[`FormatConfig`](https://docs.rs/syntaqlite/latest/syntaqlite/fmt/struct.FormatConfig.html)
(defined in
[`syntaqlite/src/fmt/mod.rs`](https://github.com/LalitMaganti/syntaqlite/blob/main/syntaqlite/src/fmt/mod.rs)):

```rust
use syntaqlite::{Formatter, FormatConfig, KeywordCase};

let config = FormatConfig::default()
    .with_line_width(120)
    .with_indent_width(4)
    .with_keyword_case(KeywordCase::Lower)
    .with_semicolons(false);

let mut fmt = Formatter::with_config(&config);
```

## C API

The C FFI exposes the same options via
[`SyntaqliteFormatConfig`](https://github.com/LalitMaganti/syntaqlite/blob/main/syntaqlite/src/fmt/ffi.rs):

```c
SyntaqliteFormatConfig config = {
    .line_width = 120,
    .indent_width = 4,
    .keyword_case = 1,   // 0 = upper, 1 = lower
    .semicolons = 1,     // 0 = false, non-zero = true
};
```

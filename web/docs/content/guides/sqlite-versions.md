+++
title = "SQLite version and compile flags"
description = "Pin validation to your production SQLite version and compile-time flags."
weight = 5
+++

# SQLite version and compile flags

Different environments run different SQLite builds. A function that works on
your development machine might not exist in production if the SQLite version is
older or was compiled without certain flags.

syntaqlite can catch these mismatches.

## Version pinning

Restrict syntaqlite to only accept syntax available in a specific SQLite
version:

```bash
syntaqlite validate --sqlite-version 3.35.0 query.sql
```

For example, the `RETURNING` clause was added in 3.35.0. If you pin to 3.34.0,
syntaqlite will reject it:

```bash
syntaqlite validate --sqlite-version 3.34.0 query.sql
```

Use `latest` (the default) to allow all known syntax.

## Compile-time flags

SQLite has compile-time options that enable or disable features. The most
common ones:

| Flag | What it enables |
|------|----------------|
| `SQLITE_ENABLE_MATH_FUNCTIONS` | `sin()`, `cos()`, `log()`, `pow()`, etc. |
| `SQLITE_ENABLE_ORDERED_SET_AGGREGATES` | `percentile_cont()`, `percentile_disc()`, `mode()`, `median()` |
| `SQLITE_ENABLE_JSON1` | `json()`, `json_extract()`, etc. |

Tell syntaqlite which flags your production build has:

```bash
syntaqlite validate \
  --sqlite-cflag SQLITE_ENABLE_MATH_FUNCTIONS \
  --sqlite-cflag SQLITE_ENABLE_JSON1 \
  query.sql
```

Functions gated behind a flag you didn't specify will be flagged as unknown.

## Combining version and flags

You can use both together:

```bash
syntaqlite validate \
  --sqlite-version 3.41.0 \
  --sqlite-cflag SQLITE_ENABLE_MATH_FUNCTIONS \
  query.sql
```

This matches a real deployment: "we run SQLite 3.41.0 compiled with math
functions."

## Formatting with version constraints

The `--sqlite-version` and `--sqlite-cflag` flags also work with `syntaqlite
fmt` and `syntaqlite lsp`, ensuring the formatter and language server match
your target environment.

+++
title = "C API reference"
description = "FFI function signatures and memory model."
weight = 4
+++

# C API reference

## Formatter functions

| Function | Description |
|----------|-------------|
| `syntaqlite_formatter_create_sqlite()` | Create with defaults |
| `syntaqlite_formatter_create_sqlite_with_config(config)` | Create with custom config |
| `syntaqlite_formatter_format(f, sql, len)` | Format SQL. Returns 0 on success, -1 on error |
| `syntaqlite_formatter_output(f)` | Pointer to formatted output |
| `syntaqlite_formatter_output_len(f)` | Length of formatted output |
| `syntaqlite_formatter_error_msg(f)` | Error message (after a failed format) |
| `syntaqlite_formatter_destroy(f)` | Free the formatter |

## Validator functions

| Function | Description |
|----------|-------------|
| `syntaqlite_validator_create_sqlite()` | Create a validator |
| `syntaqlite_validator_set_mode(v, mode)` | Set Document or Execute mode |
| `syntaqlite_validator_add_tables(v, tables, count)` | Register schema tables |
| `syntaqlite_validator_analyze(v, sql, len)` | Analyze SQL, returns diagnostic count |
| `syntaqlite_validator_diagnostic_count(v)` | Number of diagnostics from last analyze |
| `syntaqlite_validator_diagnostics(v)` | Pointer to diagnostic array |
| `syntaqlite_validator_render_diagnostics(v, filename)` | Render diagnostics as text (caller must free with `syntaqlite_string_destroy`) |
| `syntaqlite_validator_reset_catalog(v)` | Clear registered schema |
| `syntaqlite_validator_destroy(v)` | Free the validator |

## Analysis modes

| Mode | Value | Behavior |
|------|-------|----------|
| `SYNTAQLITE_MODE_DOCUMENT` | 0 | DDL resets between analyze calls (for file editing) |
| `SYNTAQLITE_MODE_EXECUTE` | 1 | DDL accumulates across calls (for interactive sessions) |

## Memory model

- Formatter and validator handles are **reusable** — create once, call
  repeatedly.
- Output strings from `syntaqlite_formatter_output()` and
  `syntaqlite_validator_diagnostics()` are **borrowed** — valid until the next
  call to format/analyze or destroy.
- Strings from `syntaqlite_validator_render_diagnostics()` are **owned** — you
  must free them with `syntaqlite_string_destroy()`.
- Passing NULL to any destroy function is a safe no-op.

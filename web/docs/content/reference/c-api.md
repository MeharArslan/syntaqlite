+++
title = "C API reference"
description = "FFI function signatures and memory model."
weight = 4
+++

# C API reference

## Formatter

### Types

```c
// Opaque formatter handle.
typedef struct SyntaqliteFormatter SyntaqliteFormatter;

// Keyword casing options.
typedef enum {
  SYNTAQLITE_KEYWORD_UPPER = 0,
  SYNTAQLITE_KEYWORD_LOWER = 1,
} SyntaqliteKeywordCase;

// Formatter configuration.
typedef struct {
  uint32_t line_width;                // Max line width before breaking (default: 80)
  uint32_t indent_width;              // Spaces per indent level (default: 2)
  SyntaqliteKeywordCase keyword_case; // Keyword casing (default: UPPER)
  uint32_t semicolons;                // Append semicolons (0 = no, nonzero = yes, default: 1)
} SyntaqliteFormatConfig;
```

Return codes from `syntaqlite_formatter_format()`:

| Constant | Value | Meaning |
|----------|-------|---------|
| `SYNTAQLITE_FORMAT_OK` | `0` | Success |
| `SYNTAQLITE_FORMAT_ERROR` | `-1` | Parse error |

### Functions

| Function | Description |
|----------|-------------|
| `syntaqlite_formatter_create_sqlite()` | Create with default config |
| `syntaqlite_formatter_create_sqlite_with_config(config)` | Create with custom `SyntaqliteFormatConfig` |
| `syntaqlite_formatter_format(f, sql, len)` | Format SQL source (`len` bytes). Returns `0` on success, `-1` on error |
| `syntaqlite_formatter_output(f)` | NUL-terminated formatted output, or `NULL` after error |
| `syntaqlite_formatter_output_len(f)` | Length of formatted output in bytes (excl. NUL) |
| `syntaqlite_formatter_error_msg(f)` | NUL-terminated error message, or `NULL` after success |
| `syntaqlite_formatter_destroy(f)` | Free the formatter. No-op if `NULL` |

## Validator

### Types

```c
// Opaque validator handle.
typedef struct SyntaqliteValidator SyntaqliteValidator;

// Diagnostic severity levels.
typedef enum {
  SYNTAQLITE_SEVERITY_ERROR   = 0,
  SYNTAQLITE_SEVERITY_WARNING = 1,
  SYNTAQLITE_SEVERITY_INFO    = 2,
  SYNTAQLITE_SEVERITY_HINT    = 3,
} SyntaqliteSeverity;

// A single diagnostic from validation.
typedef struct {
  SyntaqliteSeverity severity;
  const char* message;       // NUL-terminated, borrowed
  uint32_t start_offset;     // byte offset in source
  uint32_t end_offset;       // byte offset in source
} SyntaqliteDiagnostic;

// Table definition for batch catalog registration.
typedef struct {
  const char* name;              // NUL-terminated table name
  const char* const* columns;    // NULL = columns unknown (accepts any ref)
  uint32_t column_count;         // ignored when columns is NULL
} SyntaqliteTableDef;

// Analysis mode.
typedef enum {
  SYNTAQLITE_MODE_DOCUMENT = 0,  // DDL resets between analyze() calls
  SYNTAQLITE_MODE_EXECUTE  = 1,  // DDL accumulates across calls
} SyntaqliteAnalysisMode;
```

### Functions

| Function | Description |
|----------|-------------|
| `syntaqlite_validator_create_sqlite()` | Create a validator with default mode (`DOCUMENT`) |
| `syntaqlite_validator_set_mode(v, mode)` | Set `DOCUMENT` or `EXECUTE` mode |
| `syntaqlite_validator_add_tables(v, tables, count)` | Register schema tables from `SyntaqliteTableDef` array |
| `syntaqlite_validator_analyze(v, sql, len)` | Analyze SQL, returns diagnostic count |
| `syntaqlite_validator_diagnostic_count(v)` | Number of diagnostics from last `analyze()` |
| `syntaqlite_validator_diagnostics(v)` | Pointer to `SyntaqliteDiagnostic` array, or `NULL` if count is 0 |
| `syntaqlite_validator_render_diagnostics(v, filename)` | Render diagnostics as human-readable text. `filename` may be `NULL` (defaults to `"<input>"`) |
| `syntaqlite_validator_reset_catalog(v)` | Clear registered schema (preserves dialect builtins) |
| `syntaqlite_validator_destroy(v)` | Free the validator. No-op if `NULL` |

## Utility

| Function | Description |
|----------|-------------|
| `syntaqlite_string_destroy(s)` | Free a string returned by a `syntaqlite_*` function that documents ownership transfer. No-op if `NULL` |

## Analysis modes

| Mode | Value | Behavior |
|------|-------|----------|
| `SYNTAQLITE_MODE_DOCUMENT` | 0 | DDL resets between `analyze()` calls (for file editing) |
| `SYNTAQLITE_MODE_EXECUTE` | 1 | DDL accumulates across calls (for interactive sessions) |

## Memory model

- Formatter and validator handles are **reusable** -- create once, call
  repeatedly.
- Output strings from `syntaqlite_formatter_output()` and
  `syntaqlite_validator_diagnostics()` are **borrowed** -- valid until the next
  call to format/analyze or destroy.
- Strings from `syntaqlite_validator_render_diagnostics()` are **borrowed** --
  valid until the next `analyze()`, `render_diagnostics()`, or `destroy()` call.
- Passing `NULL` to any destroy function is a safe no-op.

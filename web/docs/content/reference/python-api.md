+++
title = "Python API reference"
description = "Functions, parameters, and return types for the syntaqlite Python library."
weight = 6
+++

# Python API reference

The Python library is a C extension (`_syntaqlite`) bundled with the pip
package. It requires Python 3.10+ and is available on macOS (arm64, x86_64),
Linux (x86_64, aarch64), and Windows (x86_64).

```python
import syntaqlite
```

If the C extension is not available (e.g. Windows arm64), the library functions
are not importable. The CLI binary is still usable.

## `syntaqlite.format_sql`

Format SQL with configurable options.

```python
syntaqlite.format_sql(sql, *, line_width=80, indent_width=2, keyword_case="upper", semicolons=True)
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `sql` | `str` | — | SQL to format |
| `line_width` | `int` | `80` | Maximum line width |
| `indent_width` | `int` | `2` | Spaces per indent level |
| `keyword_case` | `str` | `"upper"` | `"upper"` or `"lower"` |
| `semicolons` | `bool` | `True` | Append semicolons to statements |

**Returns:** `str` — the formatted SQL.

**Raises:** `syntaqlite.FormatError` — on parse error (the original SQL is syntactically invalid).

```python
>>> syntaqlite.format_sql("select 1")
'SELECT 1;\n'
>>> syntaqlite.format_sql("select 1", keyword_case="lower", semicolons=False)
'select 1\n'
```

## `syntaqlite.parse`

Parse SQL into a list of AST node dicts.

```python
syntaqlite.parse(sql)
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `sql` | `str` | SQL to parse (may contain multiple statements) |

**Returns:** `list[dict]` — one entry per statement. Each dict has:

| Key | Type | Description |
|-----|------|-------------|
| `type` | `str` | Node type name (e.g. `"SelectStmt"`, `"CreateTableStmt"`) |
| *(fields)* | `dict \| list \| str \| int \| bool` | Fields vary by node type, keyed by snake_case name |

On parse error, the entry is an error dict:

| Key | Type | Description |
|-----|------|-------------|
| `type` | `str` | `"Error"` |
| `message` | `str` | Error message |
| `offset` | `int` | Byte offset of the error |
| `length` | `int` | Length of the error span |

The parser recovers from errors and continues parsing subsequent statements.

```python
>>> syntaqlite.parse("SELECT 1")[0]["type"]
'SelectStmt'
>>> syntaqlite.parse("SELECT FROM")[0]["type"]
'Error'
```

## `syntaqlite.validate`

Validate SQL against an optional schema.

```python
syntaqlite.validate(sql, *, tables=None, render=False)
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `sql` | `str` | — | SQL to validate |
| `tables` | `list[dict] \| None` | `None` | Schema tables (see below) |
| `render` | `bool` | `False` | Return rendered diagnostics string instead of list |

Each table dict in `tables`:

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `name` | `str` | Yes | Table name |
| `columns` | `list[str] \| None` | No | Column names. `None` or omitted = accept any column. |

**Returns (render=False):** `list[dict]` — diagnostics, each with:

| Key | Type | Description |
|-----|------|-------------|
| `severity` | `str` | `"error"`, `"warning"`, `"info"`, or `"hint"` |
| `message` | `str` | Diagnostic message |
| `start_offset` | `int` | Byte offset of the start of the span |
| `end_offset` | `int` | Byte offset of the end of the span |

**Returns (render=True):** `str` — human-readable diagnostics with source
context, similar to CLI output.

```python
>>> syntaqlite.validate("SELECT 1")
[]
>>> syntaqlite.validate("SELECT nme FROM t", tables=[{"name": "t", "columns": ["name"]}])
[{'severity': 'warning', 'message': "unknown column 'nme'", ...}]
```

## `syntaqlite.tokenize`

Tokenize SQL into a list of token dicts.

```python
syntaqlite.tokenize(sql)
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `sql` | `str` | SQL to tokenize |

**Returns:** `list[dict]` — one entry per token:

| Key | Type | Description |
|-----|------|-------------|
| `text` | `str` | Token text |
| `offset` | `int` | Byte offset in the source |
| `length` | `int` | Length of the token in bytes |
| `type` | `int` | Internal token type ID |

```python
>>> syntaqlite.tokenize("SELECT 1")
[{'text': 'SELECT', 'offset': 0, 'length': 6, 'type': ...},
 {'text': '1', 'offset': 7, 'length': 1, 'type': ...}]
```

## `syntaqlite.FormatError`

Exception raised by `syntaqlite.format_sql` when the input SQL cannot be parsed.

```python
try:
    syntaqlite.format_sql("SELECT FROM")
except syntaqlite.FormatError as e:
    print(e)  # syntax error near 'FROM'
```

Inherits from `Exception`.

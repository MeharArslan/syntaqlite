+++
title = "Python library"
description = "Use syntaqlite from Python to parse, format, validate, and tokenize SQL."
weight = 6
+++

# Using syntaqlite from Python

This tutorial walks you through using syntaqlite as a Python library. By the end
you'll have a script that formats SQL, validates it against a schema, traces
column lineage, and inspects the AST — all from Python.

## 1. Install

```bash
pip install syntaqlite
```

The pip package includes both the `syntaqlite` CLI binary and a native C
extension that exposes the library API directly to Python. Requires Python 3.10+.

> **Note:** On platforms where the C extension isn't available (e.g. Windows
> arm64), pip still installs the CLI binary — only the library functions below
> won't be importable.

## 2. Format a query

```python
import syntaqlite

sql = "select id,name,email from users where active=1 order by name"
print(syntaqlite.format_sql(sql))
```

Output:

```sql
SELECT id, name, email FROM users WHERE active = 1 ORDER BY name;
```

Customize formatting with keyword arguments:

```python
print(syntaqlite.format_sql(sql, line_width=40, indent_width=4, keyword_case="lower"))
```

```sql
select
    id,
    name,
    email
from
    users
where
    active = 1
order by
    name;
```

## 3. Validate against a schema

Pass table definitions to validate column and table references:

```python
import syntaqlite
from syntaqlite import Table

schema = [Table("users", ["id", "name", "email", "active"])]
result = syntaqlite.validate(
    "SELECT nme FROM users WHERE active = 1",
    tables=schema,
)
for d in result.diagnostics:
    print(f"{d.severity}: {d.message}")
```

```text
warning: unknown column 'nme'
```

You can also load schema directly from DDL:

```python
result = syntaqlite.validate(
    "SELECT nme FROM users WHERE active = 1",
    schema_ddl="CREATE TABLE users (id INT, name TEXT, email TEXT, active INT)",
)
```

For human-readable output with source locations, use `render=True`:

```python
print(syntaqlite.validate(
    "SELECT nme FROM users WHERE active = 1",
    tables=schema,
    render=True,
))
```

```text
warning: unknown column 'nme'
 --> <expression>:1:8
  |
1 | SELECT nme FROM users WHERE active = 1
  |        ^~~
  = help: did you mean 'name'?
```

## 3b. Column lineage

When validating a SELECT, the result includes column lineage — which source
table and column each output column traces back to:

```python
result = syntaqlite.validate(
    "SELECT u.name, u.email FROM users u",
    tables=[Table("users", ["id", "name", "email"])],
)
for col in result.lineage.columns:
    print(f"  {col.name} <- {col.origin}")
```

```text
  name <- users.name
  email <- users.email
```

The `lineage` object also lists the relations referenced by the query:

```python
for rel in result.lineage.relations:
    print(f"  {rel.name} ({rel.kind})")
```

```text
  users (table)
```

## 4. Parse and inspect the AST

```python
import syntaqlite

stmts = syntaqlite.parse("SELECT 1 + 2; SELECT 'hello'")
for i, stmt in enumerate(stmts):
    print(f"--- statement {i + 1}: {stmt['type']} ---")
```

```text
--- statement 1: SelectStmt ---
--- statement 2: SelectStmt ---
```

Each statement is a nested dict. Fields use snake_case names, child nodes are
nested dicts, and lists are Python lists:

```python
stmt = syntaqlite.parse("SELECT id, name FROM users")[0]
for col in stmt["columns"]:
    expr = col["expr"]
    print(f"  {expr['type']}: {expr.get('column', expr.get('value'))}")
```

```text
  ColumnRef: id
  ColumnRef: name
```

Parse errors appear as dicts with `type: "Error"`:

```python
stmts = syntaqlite.parse("SELECT FROM; SELECT 1")
for s in stmts:
    if s["type"] == "Error":
        print(f"error at offset {s['offset']}: {s['message']}")
    else:
        print(f"ok: {s['type']}")
```

```text
error at offset 7: syntax error near 'FROM'
ok: SelectStmt
```

## 5. Tokenize

```python
import syntaqlite

for tok in syntaqlite.tokenize("SELECT 1"):
    print(f"  {tok['text']!r:10s}  offset={tok['offset']}  len={tok['length']}")
```

```text
  'SELECT'    offset=0  len=6
  '1'         offset=7  len=1
```

## Next steps

- [Python API reference](@/reference/python-api.md) — all functions, parameters,
  and return types
- [CLI reference](@/reference/cli.md) — the `syntaqlite` command installed
  alongside the library
- [Formatting options](@/reference/formatting-options.md) — line width, keyword
  casing, and more

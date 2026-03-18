# syntaqlite

Parse, format, and validate SQLite SQL from Python using SQLite's own grammar and tokenizer. No approximations: if SQLite accepts it, syntaqlite parses it.

**[Docs](https://docs.syntaqlite.com)** · **[Playground](https://playground.syntaqlite.com)** · **[GitHub](https://github.com/LalitMaganti/syntaqlite)**

```bash
pip install syntaqlite
```

Requires Python 3.10+. Wheels for Linux (x86_64, aarch64), macOS (x86_64, arm64), and Windows (x86_64).

## Library

The package includes a C extension linked directly against syntaqlite. No subprocesses, no shelling out.

### Formatting

```python
import syntaqlite

sql = "select u.id, u.name, p.title from users u join posts p on u.id = p.user_id where u.active = 1 and p.published = true order by p.created_at desc limit 10"
print(syntaqlite.format_sql(sql))
```
```
SELECT u.id, u.name, p.title
FROM users AS u
JOIN posts AS p ON u.id = p.user_id
WHERE
  u.active = 1
  AND p.published = true
ORDER BY
  p.created_at DESC
LIMIT 10;
```

Raises `syntaqlite.FormatError` on invalid input.

### Parsing

`syntaqlite.parse()` returns a full AST as typed Python objects, one per statement:

```python
import syntaqlite

stmts = syntaqlite.parse("SELECT 1 + 2 FROM foo")
stmt = stmts[0]  # SelectStmt

print(type(stmt).__name__)       # SelectStmt
print(stmt.columns[0].expr)      # BinaryExpr(...)
print(stmt.from_clause)          # TableRef(...)
print(stmt.where_clause)         # None
```

Every node type is a `__slots__` class with typed attributes, so you get IDE
autocomplete and `isinstance` checks:

```python
from syntaqlite._nodes import SelectStmt, BinaryExpr

assert isinstance(stmt, SelectStmt)
assert isinstance(stmt.columns[0].expr, BinaryExpr)
```

Enum and flag fields are wrapped as `IntEnum`/`IntFlag` from `syntaqlite._enums`:

```python
from syntaqlite._enums import BinaryOp

expr = stmt.columns[0].expr
print(BinaryOp(expr.op).name)  # PLUS
```

For performance-sensitive code, use `syntaqlite._parse_raw()` to get plain dicts
instead of typed objects:

```python
from syntaqlite._syntaqlite import parse as parse_raw
import json

stmts = parse_raw("SELECT 1 + 2; SELECT 3")
print(json.dumps(stmts[0], indent=2))
```
```json
{
  "type": "SelectStmt",
  "flags": 0,
  "columns": [
    {
      "type": "ResultColumn",
      "flags": 0,
      "alias": null,
      "expr": {
        "type": "BinaryExpr",
        "op": 0,
        "left": { "type": "Literal", "literal_type": 0, "source": "1" },
        "right": { "type": "Literal", "literal_type": 0, "source": "2" }
      }
    }
  ],
  "from_clause": null,
  "where_clause": null,
  ...
}
```

Error nodes have `"type": "Error"` with a `"message"` field.

### Tokenizing

```python
import syntaqlite

for tok in syntaqlite.tokenize("SELECT 1 + 2"):
    print(tok["text"], tok["type"])
```
```
SELECT 161
  185
1 110
  185
+ 97
  185
2 110
```

Each token is a dict with `text`, `offset`, `length`, and `type` fields.

### Validation

Check SQL against a schema without touching a database. Catches unknown tables, columns, functions, CTE column mismatches, and more.

```python
import syntaqlite

result = syntaqlite.validate(
    "SELECT nme FROM users",
    tables=[syntaqlite.Table("users", columns=["id", "name", "email"])],
)
for d in result.diagnostics:
    print(f"{d.severity}: {d.message}")
```
```
error: unknown column 'nme'
```

Pass `render=True` to get formatted diagnostics with source locations and suggestions:

```python
print(syntaqlite.validate("SELECT nme FROM users",
    tables=[syntaqlite.Table("users", columns=["id", "name", "email"])],
    render=True))
```
```
error: unknown column 'nme'
 --> <input>:1:8
  |
1 | SELECT nme FROM users
  |        ^~~
  = help: did you mean 'name'?
```

Schema can come from `syntaqlite.Table`/`syntaqlite.View` objects or raw DDL:

```python
result = syntaqlite.validate(
    "SELECT * FROM orders",
    schema_ddl="CREATE TABLE orders (id INTEGER, total REAL);",
)
```

#### Column lineage

For SELECT statements, validation results include column lineage tracing each output column back to its source:

```python
import syntaqlite

result = syntaqlite.validate(
    "SELECT id, name FROM users",
    tables=[syntaqlite.Table("users", columns=["id", "name", "email"])],
)
for col in result.lineage.columns:
    print(f"{col.name} <- {col.origin}")
```
```
id <- users.id
name <- users.name
```

## CLI

The pip package also bundles the `syntaqlite` binary:

```bash
syntaqlite fmt -e "select 1, 2, 3"
syntaqlite validate query.sql
syntaqlite parse -e "SELECT * FROM users"
```

The CLI supports pinning to a specific SQLite version or enabling compile-time flags to match your target environment:

```bash
syntaqlite --sqlite-version 3.32.0 validate query.sql
syntaqlite --sqlite-cflag SQLITE_ENABLE_MATH_FUNCTIONS validate query.sql
```

See the [CLI reference](https://docs.syntaqlite.com/main/reference/cli/) for all commands and flags.

## License

Apache 2.0. SQLite components are public domain under the [SQLite blessing](https://www.sqlite.org/copyright.html).

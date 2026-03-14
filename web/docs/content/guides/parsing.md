+++
title = "Parsing and inspecting ASTs"
description = "Parse SQL and explore the abstract syntax tree from the CLI or Rust."
weight = 4
+++

# Parsing and inspecting ASTs

syntaqlite can dump the AST for any SQL input. This is useful for debugging
queries, understanding how SQL is parsed, or building tools on top of the
parser.

## CLI: `syntaqlite parse -o ast`

```bash
echo "SELECT id, name FROM users WHERE active = 1" | syntaqlite parse -o ast
```

```
SelectStmt
  flags: (none)
  columns:
    ResultColumnList [2 items]
      ResultColumn
        flags: (none)
        alias: (none)
        expr:
          ColumnRef
            column: "id"
            table: (none)
            schema: (none)
      ResultColumn
        flags: (none)
        alias: (none)
        expr:
          ColumnRef
            column: "name"
            table: (none)
            schema: (none)
  from_clause:
    TableRef
      table_name: "users"
      schema: (none)
      alias: (none)
      args: (none)
  where_clause:
    BinaryExpr
      op: EQ
      left:
        ColumnRef
          column: "active"
          table: (none)
          schema: (none)
      right:
        Literal
          literal_type: INTEGER
          source: "1"
  groupby: (none)
  having: (none)
  orderby: (none)
  limit_clause: (none)
  window_clause: (none)
```

Each node shows its type and all named fields. Absent fields show `(none)`.
Lists show their item count. Enum fields (like `op: EQ`) show the variant
name.

For files:

```bash
syntaqlite parse -o ast schema.sql queries.sql
```

When given multiple files, each is prefixed with `==> filename <==`.

## Rust: streaming parser

The parser yields one statement at a time, so memory usage stays proportional
to the largest single statement:

```rust
use syntaqlite::{Parser, ParseOutcome};

let parser = Parser::new();
let mut session = parser.parse("SELECT 1; SELECT 2;");

loop {
    match session.next() {
        ParseOutcome::Ok(stmt) => {
            // stmt is a ParsedStatement
            let mut buf = String::new();
            stmt.dump(&mut buf, 0);
            println!("{buf}");
        }
        ParseOutcome::Err(err) => {
            eprintln!("error: {}", err.message());
            // Recoverable errors continue to the next statement.
            // Fatal errors should break the loop.
        }
        ParseOutcome::Done => break,
    }
}
```

### Accessing tokens

To get token-level information, enable token collection in the parser config:

```rust
use syntaqlite::{Parser, ParseOutcome};
use syntaqlite::parse::ParserConfig;

let config = ParserConfig::default().with_collect_tokens(true);
let parser = Parser::with_config(&config);
let mut session = parser.parse("SELECT max(x) FROM t");

if let ParseOutcome::Ok(stmt) = session.next() {
    for token in stmt.tokens() {
        println!(
            "{:4}..{:4}  {:?}  {:?}",
            token.offset(),
            token.offset() + token.length(),
            token.token_type(),
            token.text(),
        );
    }

    // Comments are separate from tokens:
    for comment in stmt.comments() {
        println!("comment at {}: {}", comment.offset(), comment.text());
    }
}
```

Token flags indicate how the parser used each token — for example,
`token.flags().used_as_function()` is `true` for `max` in `max(x)`.

### Generic traversal

For grammar-agnostic tree walking (works with any dialect), use the type-erased
API via
[`extract_fields()`](https://github.com/LalitMaganti/syntaqlite/blob/main/syntaqlite/src/fmt/interpret.rs):

```rust
use syntaqlite::any::{AnyParsedStatement, FieldValue};

fn walk(stmt: &AnyParsedStatement, node_id: u32, depth: usize) {
    if let Some((tag, fields)) = stmt.extract_fields(node_id) {
        // tag identifies the node type
        for (i, field) in fields.iter().enumerate() {
            match field {
                FieldValue::NodeId(child) => walk(stmt, *child, depth + 1),
                FieldValue::Span(text) => println!("{:indent$}{text}", "", indent = depth * 2),
                _ => {}
            }
        }
    }
}
```

This is the same traversal pattern used internally by the
[formatter's bytecode interpreter](https://github.com/LalitMaganti/syntaqlite/blob/main/syntaqlite/src/fmt/interpret.rs).

## WASM: JSON AST

The
[JavaScript API](@/getting-started/wasm-js.md)
can return the AST as structured JSON:

```typescript
const result = engine.runAstJson("SELECT 1 + 2");
if (result.ok) {
    console.log(JSON.stringify(result.statements[0], null, 2));
}
```

```json
{
  "type": "SelectStmt",
  "columns": {
    "type": "ResultColumnList",
    "count": 1,
    "children": [
      {
        "type": "ResultColumn",
        "expr": {
          "type": "BinaryExpr",
          "op": "Plus",
          "left": {"type": "Literal", "value": "1"},
          "right": {"type": "Literal", "value": "2"}
        }
      }
    ]
  }
}
```

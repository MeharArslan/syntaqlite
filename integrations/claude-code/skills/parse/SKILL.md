---
name: parse
description: Parse SQL and inspect the AST using syntaqlite. Use when the user wants to see the parse tree, debug SQL syntax, or understand how a query is structured.
---

# Parse SQL

Parse SQLite SQL and display the abstract syntax tree (AST) using the syntaqlite CLI.

## Usage

```bash
# Print AST from a file
syntaqlite parse --output text query.sql

# Print AST from stdin
echo "SELECT 1 + 2 FROM t" | syntaqlite parse --output text

# Output as JSON
syntaqlite parse --output json query.sql

# Show bytecodes (parser internals)
syntaqlite parse --output bytecode query.sql

# Show doc-tree (formatting structure)
syntaqlite parse --output doc-tree query.sql
```

## Output modes

- `text` — Human-readable indented tree (default)
- `json` — Machine-readable JSON AST
- `bytecode` — Parser bytecode sequence (debugging)
- `doc-tree` — Formatter document tree (debugging formatting)

## Notes

- Use `text` output for quick inspection and `json` for programmatic use.
- If the user is debugging why something formats incorrectly, `doc-tree` shows the formatter's internal representation.

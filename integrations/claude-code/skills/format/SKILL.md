---
name: format
description: Format SQL files using the syntaqlite formatter. Use when the user wants to format, reformat, or pretty-print SQL code.
---

# Format SQL

Format SQLite SQL using the syntaqlite CLI formatter.

## Usage

```bash
# Format a file (print to stdout)
syntaqlite fmt query.sql

# Format in-place
syntaqlite fmt -i query.sql

# Format from stdin
echo "SELECT a,b FROM t WHERE x=1" | syntaqlite fmt

# Format multiple files via glob
syntaqlite fmt -i "**/*.sql"

# Format with options
syntaqlite fmt -w 120 -k upper query.sql
```

## Options

- `-w, --line-width <N>` — max line width (default: 80)
- `-k, --keyword-case <upper|lower>` — keyword casing (default: upper)
- `-i, --in-place` — overwrite files in place
- `--semicolons <true|false>` — append semicolons (default: true)

## Notes

- When the user asks to format a specific file, use `-i` to write in place.
- When formatting multiple files, use a glob pattern with `-i`.
- To preview formatting without modifying files, omit `-i`.

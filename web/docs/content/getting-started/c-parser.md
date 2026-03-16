+++
title = "C parser"
description = "Parse SQL from C using the source amalgamation."
weight = 4
+++

# Parse SQL from C

This tutorial walks you through parsing a SQL query from C using the
syntaqlite source amalgamation. By the end you'll have a small program that
parses a query and prints its AST — no dependencies beyond a C compiler.

## 1. Download the amalgamation

The source amalgamation is two files: `syntaqlite_syntax.h` (header) and
`syntaqlite_syntax.c` (implementation). Download them from the latest release:

```bash
mkdir sql-parser && cd sql-parser
curl -L https://github.com/LalitMaganti/syntaqlite/releases/latest/download/syntaqlite-syntax-amalgamation.tar.gz | tar xz
```

You should have:

```bash
ls
# syntaqlite_syntax.c  syntaqlite_syntax.h
```

## 2. Write the program

Create `parse.c`:

```c
#include "syntaqlite_syntax.h"
#include <stdio.h>
#include <string.h>

int main(int argc, char** argv) {
    const char* sql = "SELECT id, name FROM users WHERE active = 1";
    if (argc > 1) {
        sql = argv[1];
    }

    // Create a parser for the SQLite dialect.
    SyntaqliteParser* p = syntaqlite_parser_create(NULL);

    // Feed the source text.
    syntaqlite_parser_reset(p, sql, strlen(sql));

    // Parse each statement (a source string can contain multiple).
    int stmt = 0;
    int rc;
    while ((rc = syntaqlite_parser_next(p)) != -1) {
        stmt++;
        if (rc != 0) {
            fprintf(stderr, "error in statement %d: %s\n",
                stmt, syntaqlite_result_error_msg(p));
            continue;
        }

        // Print the AST.
        uint32_t root = syntaqlite_result_root(p);
        char* dump = syntaqlite_dump_node(p, root, 0);
        printf("--- statement %d ---\n%s\n", stmt, dump);
        free(dump);
    }

    syntaqlite_parser_destroy(p);
    return 0;
}
```

## 3. Compile and run

```bash
cc -O2 -o parse parse.c syntaqlite_syntax.c
```

Run it with the default query:

```bash
./parse
```

```text
--- statement 1 ---
SelectStmt
  columns:
    ResultColumn
      expr:
        ColumnRef
          column: "id"
    ResultColumn
      expr:
        ColumnRef
          column: "name"
  from_clause:
    TableRef
      table_name: "users"
  where_clause:
    BinaryExpr
      op: EQ
      ...
```

Pass your own SQL:

```bash
./parse "CREATE TABLE t(x INTEGER PRIMARY KEY, y TEXT NOT NULL)"
```

## 4. Handle errors

The parser recovers from errors and keeps going. Try invalid SQL:

```bash
./parse "SELECT FROM; SELECT 1"
```

```text
error in statement 1: syntax error near 'FROM'
--- statement 2 ---
SelectStmt
  columns:
    ResultColumn
      expr:
        Literal: 1
```

Statement 1 failed but statement 2 still parsed successfully.

## 5. Access tokens

Enable the token side-table to see what the tokenizer produced:

```c
syntaqlite_parser_set_collect_tokens(p, 1);
syntaqlite_parser_reset(p, sql, strlen(sql));
syntaqlite_parser_next(p);

uint32_t count;
const SyntaqliteParserToken* tokens =
    syntaqlite_result_tokens(p, &count);
for (uint32_t i = 0; i < count; i++) {
    printf("token %u: type=%u offset=%u len=%u\n",
        i, tokens[i].token_type,
        tokens[i].span.offset, tokens[i].span.length);
}
```

## Next steps

- The source amalgamation gives you the **parser and tokenizer**. For the
  **formatter and validator**, use the [prebuilt shared library](@/guides/c-api.md#option-2-prebuilt-shared-library-full-api).
- See the [C API reference](@/reference/c-api.md) for the full list of
  parser, tokenizer, formatter, and validator functions.

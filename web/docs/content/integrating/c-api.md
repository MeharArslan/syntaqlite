+++
title = "C"
description = "Link syntaqlite into a C or C++ project."
weight = 2
+++

# Using syntaqlite from C

syntaqlite provides a C FFI for embedding the formatter and validator in
non-Rust projects. Link against `libsyntaqlite`.

## Format a query

```c
#include <string.h>
#include <stdio.h>

// Link against libsyntaqlite

int main(void) {
    SyntaqliteFormatter* f = syntaqlite_formatter_create_sqlite();

    const char* sql = "select a,b from t where x=1";
    if (syntaqlite_formatter_format(f, sql, strlen(sql)) == 0) {
        printf("%.*s\n",
            syntaqlite_formatter_output_len(f),
            syntaqlite_formatter_output(f));
    }

    syntaqlite_formatter_destroy(f);
    return 0;
}
```

The formatter handle is reusable — create once, call `format()` repeatedly.
Output is borrowed and valid until the next call to format or destroy.

## Validate a query

```c
SyntaqliteValidator* v = syntaqlite_validator_create_sqlite();

// Register your schema
SyntaqliteTableDef tables[] = {
    {"users",  (const char*[]){"id", "name", "email"}, 3},
    {"posts",  (const char*[]){"id", "user_id", "body"}, 3},
};
syntaqlite_validator_add_tables(v, tables, 2);

// Analyze
const char* sql = "SELECT nme FROM users";
syntaqlite_validator_analyze(v, sql, strlen(sql));

// Print human-readable diagnostics
const char* rendered = syntaqlite_validator_render_diagnostics(v, "query.sql");
printf("%s\n", rendered);
syntaqlite_string_destroy((char*)rendered);

syntaqlite_validator_destroy(v);
```

## Next steps

- See the [C API reference](@/reference/c-api.md) for all functions and the
  memory model

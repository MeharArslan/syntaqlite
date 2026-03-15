+++
title = "Rust"
description = "Add syntaqlite to a Rust project and format your first query."
weight = 1
+++

# Using syntaqlite from Rust

## Add the dependency

```toml
[dependencies]
syntaqlite = { version = "0.0.7", features = ["fmt"] }
```

## Format a query

```rust
use syntaqlite::Formatter;

let mut fmt = Formatter::new();
let output = fmt.format("select a,b from t where x=1")?;
println!("{output}");
// SELECT a, b
// FROM t
// WHERE x = 1;
```

That's it — `Formatter::new()` uses sensible defaults (80-char lines, 2-space
indent, uppercase keywords). The formatter is reusable: call `format()`
repeatedly and internal allocations are recycled.

## Customize formatting

```rust
use syntaqlite::{Formatter, FormatConfig, KeywordCase};

let config = FormatConfig::default()
    .with_line_width(120)
    .with_indent_width(4)
    .with_keyword_case(KeywordCase::Lower);
let mut fmt = Formatter::with_config(&config);
let output = fmt.format("SELECT 1")?;
```

## Parse SQL

If you need the AST directly:

```rust
use syntaqlite_syntax::{Parser, ParseOutcome};

let parser = Parser::new();
let mut session = parser.parse("SELECT 1; SELECT 2;");

loop {
    match session.next() {
        ParseOutcome::Ok(stmt) => {
            // Process the parsed statement
        }
        ParseOutcome::Err(err) => {
            eprintln!("Parse error: {}", err.message());
            break;
        }
        ParseOutcome::Done => break,
    }
}
```

## Next steps

- See the [Rust API reference](@/reference/rust-api.md) for all types and
  methods
- Enable the `validation` feature for semantic analysis

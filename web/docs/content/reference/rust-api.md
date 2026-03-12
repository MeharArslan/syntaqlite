+++
title = "Rust API reference"
description = "Feature flags, types, and methods."
weight = 3
+++

# Rust API reference

## Feature flags

| Feature | What it enables |
|---------|----------------|
| `fmt` | Formatter |
| `validation` | Semantic analysis (unknown tables, columns, functions) |
| `sqlite` | Built-in SQLite dialect (enabled by default) |
| `lsp` | Language server protocol implementation |
| `serde-json` | JSON serialization for AST and diagnostics |
| `dynload` | Load custom dialects from shared libraries at runtime |

## Formatter

| Type / Method | Description |
|---------------|-------------|
| `Formatter::new()` | Create with SQLite dialect and default settings |
| `Formatter::with_config(&FormatConfig)` | Create with custom config |
| `fmt.format(sql) -> Result<String>` | Format SQL string |
| `FormatConfig` | `line_width`, `indent_width`, `keyword_case`, `semicolons` |
| `KeywordCase` | `Upper` or `Lower` |

The formatter is reusable — call `format()` repeatedly. Internal allocations
are reused across calls. Defaults: 80-char lines, 2-space indent, uppercase
keywords, semicolons on.

## Parser

| Type / Method | Description |
|---------------|-------------|
| `Parser::new()` | Create a parser for the SQLite grammar |
| `parser.parse(sql) -> ParseSession` | Start a parse session |
| `session.next() -> ParseOutcome` | Yield next statement |
| `ParseOutcome::Ok(stmt)` | Successfully parsed statement |
| `ParseOutcome::Err(err)` | Parse error (parser recovers and continues) |
| `ParseOutcome::Done` | No more statements |

The parser yields statements one at a time, reusing internal allocations.
It recovers from errors and continues parsing subsequent statements.

## Tokenizer

| Type / Method | Description |
|---------------|-------------|
| `Tokenizer::new()` | Create a tokenizer for the SQLite grammar |
| `tokenizer.tokenize(sql) -> Result<Vec<Token>>` | Tokenize SQL string |
| `Token` | `token_type`, `text`, byte offsets into source |

Zero-copy — tokens reference byte offsets into the source string.

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use crate::pipeline::writers::rust_writer::RustWriter;

const LIB_MODULE_DECLS: &str = r#"
mod ffi;
/// Typed AST nodes for this dialect.
///
/// Each SQL statement type (e.g. `SELECT`, `INSERT`) has a corresponding struct
/// with typed accessors for its fields. The top-level enum is [`ast::Stmt`],
/// returned by [`StatementCursor::next_statement`] and
/// [`LowLevelCursor::finish`](low_level::LowLevelCursor::finish).
pub mod ast;
mod wrappers;
"#;

const LIB_LOW_LEVEL_MOD: &str = r#"
/// Low-level APIs for advanced use cases (e.g. custom token feeding/tokenizing).
pub mod low_level {
    pub use crate::wrappers::{LowLevelCursor, LowLevelParser, Tokenizer, TokenCursor};
    pub use crate::tokens::TokenType;

    /// Access the dialect handle (for use with `syntaqlite_runtime` APIs).
    pub fn dialect() -> &'static syntaqlite_runtime::Dialect<'static> {
        &crate::DIALECT
    }
}
"#;

const LIB_EXPORTS: &str = r#"
pub use wrappers::{Formatter, Parser, StatementCursor};
pub use syntaqlite_runtime::ParseError;
"#;

const LIB_CONFIG_MOD: &str = r#"
/// Configuration types for parsers and formatters.
pub mod config {
    pub use syntaqlite_runtime::fmt::{FormatConfig, KeywordCase};
    pub use syntaqlite_runtime::parser::ParserConfig;
}
"#;

const WRAPPERS_PRELUDE: &str = r#"
use std::ops::Range;

use crate::ast::{FromArena, Stmt};
use crate::low_level::TokenType;
use crate::ParseError;
"#;

const WRAPPER_PARSER: &str = r#"
/// A parser pre-configured for this dialect.
///
/// Returns typed `StatementCursor` wrappers from `parse()`.
pub struct Parser {
    inner: syntaqlite_runtime::Parser,
}

impl Parser {
    /// Create a new parser with default configuration.
    pub fn new() -> Self {
        Parser {
            inner: syntaqlite_runtime::Parser::new(&crate::DIALECT),
        }
    }

    /// Create a parser with the given configuration.
    pub fn with_config(config: &syntaqlite_runtime::parser::ParserConfig) -> Self {
        Parser {
            inner: syntaqlite_runtime::Parser::with_config(&crate::DIALECT, config),
        }
    }

    /// Access the current configuration.
    pub fn config(&self) -> &syntaqlite_runtime::parser::ParserConfig {
        self.inner.config()
    }

    /// Parse source text and return a `StatementCursor` for iterating statements.
    pub fn parse<'a>(&'a mut self, source: &'a str) -> StatementCursor<'a> {
        StatementCursor { inner: self.inner.parse(source) }
    }
}
"#;

const WRAPPER_STATEMENT_CURSOR: &str = r#"
/// A high-level parsing cursor with typed node access.
pub struct StatementCursor<'a> {
    inner: syntaqlite_runtime::StatementCursor<'a>,
}

impl<'a> StatementCursor<'a> {
    /// Parse and return the next SQL statement as a typed `Stmt`.
    ///
    /// The returned `Stmt` borrows this cursor, so it cannot outlive it.
    /// Returns `None` when all statements have been consumed.
    pub fn next_statement(&mut self) -> Option<Result<Stmt<'_>, ParseError>> {
        let id = match self.inner.next_statement()? {
            Ok(id) => id,
            Err(e) => return Some(Err(e)),
        };
        let reader = self.inner.reader();
        Some(Ok(Stmt::from_arena(reader, id).expect("parser returned invalid node")))
    }
}
"#;

const WRAPPER_LOW_LEVEL_PARSER: &str = r#"
/// A low-level parser for token-by-token feeding.
///
/// Feed tokens one at a time via `LowLevelCursor`.
pub struct LowLevelParser {
    inner: syntaqlite_runtime::parser::LowLevelParser,
}

impl LowLevelParser {
    /// Create a new low-level parser with default configuration.
    pub fn new() -> Self {
        LowLevelParser {
            inner: syntaqlite_runtime::parser::LowLevelParser::new(&crate::DIALECT),
        }
    }

    /// Create a low-level parser with the given configuration.
    pub fn with_config(config: &syntaqlite_runtime::parser::ParserConfig) -> Self {
        LowLevelParser {
            inner: syntaqlite_runtime::parser::LowLevelParser::with_config(&crate::DIALECT, config),
        }
    }

    /// Bind source text and return a `LowLevelCursor` for token feeding.
    pub fn feed<'a>(&'a mut self, source: &'a str) -> LowLevelCursor<'a> {
        LowLevelCursor { inner: self.inner.feed(source) }
    }
}
"#;

const WRAPPER_LOW_LEVEL_CURSOR: &str = r#"
/// A low-level cursor for feeding tokens one at a time.
///
/// After calling `finish()`, no further feeding methods may be called.
pub struct LowLevelCursor<'a> {
    inner: syntaqlite_runtime::parser::LowLevelCursor<'a>,
}

impl<'a> LowLevelCursor<'a> {
    /// Feed a typed token to the parser.
    ///
    /// Returns `Ok(Some(stmt))` when a statement completes,
    /// `Ok(None)` to keep going, or `Err` on parse error.
    ///
    /// The returned `Stmt` borrows this cursor, so it cannot be held
    /// across further `feed_token` calls.
    ///
    /// `span` is a byte range into the source text bound by this cursor.
    pub fn feed_token(
        &mut self,
        token_type: TokenType,
        span: Range<usize>,
    ) -> Result<Option<Stmt<'_>>, ParseError> {
        match self.inner.feed_token(token_type.into(), span)? {
            None => Ok(None),
            Some(id) => {
                let reader = self.inner.base().reader();
                Ok(Some(Stmt::from_arena(reader, id).expect("parser returned invalid node")))
            }
        }
    }

    /// Signal end of input.
    ///
    /// Returns `Ok(Some(stmt))` if a final statement completed,
    /// `Ok(None)` if there was nothing pending, or `Err` on parse error.
    ///
    /// After calling `finish()`, no further feeding methods may be called.
    pub fn finish(&mut self) -> Result<Option<Stmt<'_>>, ParseError> {
        match self.inner.finish()? {
            None => Ok(None),
            Some(id) => {
                let reader = self.inner.base().reader();
                Ok(Some(Stmt::from_arena(reader, id).expect("parser returned invalid node")))
            }
        }
    }

    /// Mark subsequent fed tokens as being inside a macro expansion.
    pub fn begin_macro(&mut self, call_offset: u32, call_length: u32) {
        self.inner.begin_macro(call_offset, call_length)
    }

    /// End the innermost macro expansion region.
    pub fn end_macro(&mut self) {
        self.inner.end_macro()
    }
}
"#;

const WRAPPER_FORMATTER: &str = r#"
/// SQL formatter pre-configured for this dialect.
pub struct Formatter {
    inner: syntaqlite_runtime::fmt::Formatter<'static>,
}

impl Formatter {
    /// Create a formatter with default configuration.
    pub fn new() -> Result<Self, &'static str> {
        let inner = syntaqlite_runtime::fmt::Formatter::new(&crate::DIALECT)?;
        Ok(Formatter { inner })
    }

    /// Create a formatter with the given configuration.
    pub fn with_config(config: crate::config::FormatConfig) -> Result<Self, &'static str> {
        let inner = syntaqlite_runtime::fmt::Formatter::with_config(&crate::DIALECT, config)?;
        Ok(Formatter { inner })
    }

    /// Access the current configuration.
    pub fn config(&self) -> &crate::config::FormatConfig {
        self.inner.config()
    }

    /// Format SQL source text.
    pub fn format(
        &mut self,
        source: &str,
    ) -> Result<String, ParseError> {
        self.inner.format(source)
    }
}
"#;

const WRAPPER_TOKENIZER: &str = r#"
/// A tokenizer for SQL.
pub struct Tokenizer {
    inner: syntaqlite_runtime::parser::Tokenizer,
}

impl Tokenizer {
    /// Create a new tokenizer.
    pub fn new() -> Self {
        Tokenizer {
            inner: syntaqlite_runtime::parser::Tokenizer::new(*crate::DIALECT),
        }
    }

    /// Bind source text and return a cursor for iterating typed tokens.
    pub fn tokenize<'a>(&'a mut self, source: &'a str) -> TokenCursor<'a> {
        TokenCursor {
            inner: self.inner.tokenize(source),
        }
    }

    /// Zero-copy variant: bind a null-terminated source and return a
    /// `TokenCursor`. The source must be valid UTF-8 (panics otherwise).
    pub fn tokenize_cstr<'a>(&'a mut self, source: &'a std::ffi::CStr) -> TokenCursor<'a> {
        TokenCursor {
            inner: self.inner.tokenize_cstr(source),
        }
    }
}
"#;

const WRAPPER_TOKEN_CURSOR: &str = r#"
/// An active tokenizer session yielding typed tokens.
pub struct TokenCursor<'a> {
    inner: syntaqlite_runtime::parser::TokenCursor<'a>,
}

impl<'a> Iterator for TokenCursor<'a> {
    type Item = (TokenType, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let raw = self.inner.next()?;
        let tt = TokenType::from_raw(raw.token_type)
            .unwrap_or(TokenType::Illegal);
        Some((tt, raw.text))
    }
}
"#;

fn emit_section(w: &mut RustWriter, section: &str) {
    w.lines(section);
    w.newline();
}

fn emit_lib_dialect_binding(w: &mut RustWriter, dialect_fn: &str) {
    w.line("use std::sync::LazyLock;");
    w.newline();
    w.line("use syntaqlite_runtime::dialect::ffi as dialect_ffi;");
    w.line("unsafe extern \"C\" {");
    w.indent();
    w.line(&format!(
        "fn {}() -> *const dialect_ffi::Dialect;",
        dialect_fn
    ));
    w.dedent();
    w.line("}");
    w.newline();
    w.line("static DIALECT: LazyLock<syntaqlite_runtime::Dialect<'static>> =");
    w.line(&format!(
        "    LazyLock::new(|| unsafe {{ syntaqlite_runtime::Dialect::from_raw({}()) }});",
        dialect_fn
    ));
    w.newline();
}

pub fn generate_rust_lib(dialect_fn: &str) -> String {
    let mut w = RustWriter::new();
    w.file_header();
    emit_section(&mut w, LIB_MODULE_DECLS);
    emit_lib_dialect_binding(&mut w, dialect_fn);
    emit_section(&mut w, LIB_LOW_LEVEL_MOD);
    emit_section(&mut w, LIB_EXPORTS);
    emit_section(&mut w, LIB_CONFIG_MOD);
    w.line("mod tokens;");
    w.finish()
}

/// Generate `wrappers.rs` for a dialect crate.
pub fn generate_rust_wrappers() -> String {
    let mut w = RustWriter::new();
    w.file_header();
    emit_section(&mut w, WRAPPERS_PRELUDE);
    emit_section(&mut w, WRAPPER_PARSER);
    emit_section(&mut w, WRAPPER_STATEMENT_CURSOR);
    emit_section(&mut w, WRAPPER_LOW_LEVEL_PARSER);
    emit_section(&mut w, WRAPPER_LOW_LEVEL_CURSOR);
    emit_section(&mut w, WRAPPER_FORMATTER);
    emit_section(&mut w, WRAPPER_TOKENIZER);
    w.lines(WRAPPER_TOKEN_CURSOR);
    w.finish()
}

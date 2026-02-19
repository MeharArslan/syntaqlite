// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::ops::Range;

use crate::ast::{FromArena, Stmt};
use crate::low_level::TokenType;
use crate::ParseError;

// ── Parser ──────────────────────────────────────────────────────────────

/// A parser pre-configured for the SQLite dialect.
///
/// Returns typed `StatementCursor` wrappers from `parse()`.
pub struct Parser {
    inner: syntaqlite_runtime::Parser,
}

impl Parser {
    /// Create a new parser for the SQLite dialect with default configuration.
    pub fn new() -> Self {
        Parser {
            inner: syntaqlite_runtime::Parser::new(crate::dialect()),
        }
    }

    /// Create a parser with the given configuration.
    pub fn with_config(config: &syntaqlite_runtime::parser::ParserConfig) -> Self {
        Parser {
            inner: syntaqlite_runtime::Parser::with_config(crate::dialect(), config),
        }
    }

    /// Parse source text and return a `StatementCursor` for iterating statements.
    pub fn parse<'a>(&'a mut self, source: &'a str) -> StatementCursor<'a> {
        StatementCursor { inner: self.inner.parse(source) }
    }
}

// ── StatementCursor ─────────────────────────────────────────────────────

/// A high-level parsing cursor with typed SQLite node access.
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

// ── LowLevelParser ─────────────────────────────────────────────────────

/// A low-level parser pre-configured for the SQLite dialect.
///
/// Feed tokens one at a time via `LowLevelCursor`.
pub struct LowLevelParser {
    inner: syntaqlite_runtime::parser::LowLevelParser,
}

impl LowLevelParser {
    /// Create a new low-level parser for the SQLite dialect with default configuration.
    pub fn new() -> Self {
        LowLevelParser {
            inner: syntaqlite_runtime::parser::LowLevelParser::new(crate::dialect()),
        }
    }

    /// Create a low-level parser with the given configuration.
    pub fn with_config(config: &syntaqlite_runtime::parser::ParserConfig) -> Self {
        LowLevelParser {
            inner: syntaqlite_runtime::parser::LowLevelParser::with_config(crate::dialect(), config),
        }
    }

    /// Bind source text and return a `LowLevelCursor` for token feeding.
    pub fn feed<'a>(&'a mut self, source: &'a str) -> LowLevelCursor<'a> {
        LowLevelCursor { inner: self.inner.feed(source) }
    }
}

// ── LowLevelCursor ──────────────────────────────────────────────────────

/// A low-level cursor for feeding tokens one at a time.
///
/// After calling `finish()`, only `node()` and `base()` may be called.
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

// ── Formatter ───────────────────────────────────────────────────────────

/// SQL formatter pre-configured for the SQLite dialect.
pub struct Formatter {
    inner: syntaqlite_runtime::fmt::Formatter<'static>,
}

impl Formatter {
    /// Create a formatter with default configuration.
    pub fn new() -> Result<Self, &'static str> {
        let inner = syntaqlite_runtime::fmt::Formatter::new(crate::dialect())?;
        Ok(Formatter { inner })
    }

    /// Create a formatter with the given configuration.
    pub fn with_config(config: crate::FormatConfig) -> Result<Self, &'static str> {
        let inner = syntaqlite_runtime::fmt::Formatter::with_config(crate::dialect(), config)?;
        Ok(Formatter { inner })
    }

    /// Access the current configuration.
    pub fn config(&self) -> &crate::FormatConfig {
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

// ── Tokenizer ───────────────────────────────────────────────────────────

/// A tokenizer for SQLite SQL.
pub struct Tokenizer {
    inner: syntaqlite_runtime::parser::Tokenizer,
}

impl Tokenizer {
    /// Create a new tokenizer.
    pub fn new() -> Self {
        Tokenizer {
            inner: syntaqlite_runtime::parser::Tokenizer::new(),
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

// ── TokenCursor ────────────────────────────────────────────────────

/// An active tokenizer session yielding typed SQLite tokens.
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

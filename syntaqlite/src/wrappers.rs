// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::ops::Range;

use syntaqlite_runtime::parser::{Comment, CursorBase, MacroRegion};

use crate::ast::{FromArena, Node, Stmt};
use crate::low_level::TokenType;
use crate::NodeId;
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

    /// Enable parser trace output (prints state transitions to stderr).
    pub fn with_trace(mut self) -> Self {
        self.inner.set_trace(true);
        self
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

    /// Access the underlying `CursorBase` (e.g. for `Formatter::format_node`).
    pub(crate) fn base(&self) -> &CursorBase<'a> {
        self.inner.base()
    }
}

// ── TokenParser ─────────────────────────────────────────────────────────

/// A low-level token parser pre-configured for the SQLite dialect.
pub struct TokenParser {
    inner: syntaqlite_runtime::parser::TokenParser,
}

impl TokenParser {
    /// Create a new token parser for the SQLite dialect.
    pub fn new() -> Self {
        TokenParser {
            inner: syntaqlite_runtime::parser::TokenParser::new(crate::dialect()),
        }
    }

    /// Enable parser trace output (prints state transitions to stderr).
    pub fn with_trace(mut self) -> Self {
        self.inner.set_trace(true);
        self
    }

    /// Enable token collection (needed for comment capture).
    pub fn with_collect_tokens(mut self) -> Self {
        self.inner.set_collect_tokens(true);
        self
    }

    /// Bind source text and return a `TokenFeeder` for low-level token feeding.
    pub fn feed<'a>(&'a mut self, source: &'a str) -> TokenFeeder<'a> {
        TokenFeeder { inner: self.inner.feed(source) }
    }
}

// ── TokenFeeder ─────────────────────────────────────────────────────────

/// A low-level token-feeding cursor with typed SQLite node/token access.
pub struct TokenFeeder<'a> {
    inner: syntaqlite_runtime::parser::TokenFeeder<'a>,
}

impl<'a> TokenFeeder<'a> {
    /// Feed a typed token to the parser.
    ///
    /// `span` is a byte range into the source text bound by this feeder.
    pub fn feed_token(
        &mut self,
        token_type: TokenType,
        span: Range<usize>,
    ) -> Result<Option<NodeId>, ParseError> {
        self.inner.feed_token(token_type.into(), span)
    }

    /// Signal end of input.
    pub fn finish(
        &mut self,
    ) -> Result<Option<NodeId>, ParseError> {
        self.inner.finish()
    }

    /// Mark subsequent fed tokens as being inside a macro expansion.
    pub fn begin_macro(&mut self, call_offset: u32, call_length: u32) {
        self.inner.begin_macro(call_offset, call_length)
    }

    /// End the innermost macro expansion region.
    pub fn end_macro(&mut self) {
        self.inner.end_macro()
    }

    /// Get a typed AST node by ID.
    ///
    /// The returned `Node` borrows this feeder, so it cannot outlive it.
    pub fn node(&self, id: NodeId) -> Option<Node<'_>> {
        Node::resolve(self.inner.reader(), id)
    }

    /// Return all comments captured during parsing.
    pub fn comments(&self) -> &[Comment] {
        self.inner.comments()
    }

    /// Return all macro regions.
    pub fn macro_regions(&self) -> &[MacroRegion] {
        self.inner.macro_regions()
    }

    /// Access the underlying `CursorBase` (e.g. for `Formatter::format_node`).
    pub fn base(&self) -> &CursorBase<'a> {
        self.inner.base()
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

    /// Set the format configuration.
    pub fn with_config(mut self, config: crate::FormatConfig) -> Self {
        self.inner = self.inner.with_config(config);
        self
    }

    /// Set whether to append semicolons after each statement.
    pub fn with_semicolons(mut self, semicolons: bool) -> Self {
        self.inner = self.inner.with_semicolons(semicolons);
        self
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

    /// Format a single pre-parsed AST node.
    pub fn format_node(
        &self,
        cursor: &StatementCursor<'_>,
        node_id: NodeId,
    ) -> String {
        self.inner.format_node(cursor.base(), node_id)
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

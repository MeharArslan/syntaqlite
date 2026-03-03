// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Concrete SQLite API types.
//!
//! These types bake in the SQLite dialect, providing a clean API
//! without type parameters.  For dialect-generic access, see the
//! [`dialect`](crate::dialect) module.

use std::ops::Range;

use syntaqlite_parser::{
    Comment, MacroRegion, NodeRef, ParseError, ParserConfig, RawNodeId,
    TypedDialectEnv as TaggedDialect,
};
use syntaqlite_parser_sqlite::SqliteNodeFamily;

use crate::parser::typed::{
    DialectIncrementalCursor, DialectIncrementalParser, DialectParser, DialectStatementCursor,
    DialectTokenCursor, DialectTokenizer,
};

// ── Parser ───────────────────────────────────────────────────────────────

/// A SQL parser for the built-in SQLite dialect.
///
/// Wraps the generic [`DialectParser`](crate::dialect::DialectParser) with
/// the SQLite node family baked in so call sites never need type parameters.
///
/// # Example
///
/// ```
/// use syntaqlite::Parser;
///
/// let parser = Parser::new();
/// let mut cursor = parser.parse("SELECT 1 + 2; CREATE TABLE t(x)");
/// while let Some(result) = cursor.next_statement() {
///     let stmt = result.expect("parse error");
///     println!("{stmt:?}");
/// }
/// ```
pub struct Parser {
    inner: DialectParser<'static, SqliteNodeFamily>,
}

impl Parser {
    /// Create a parser for the built-in SQLite dialect with default configuration.
    pub fn new() -> Self {
        Parser {
            inner: DialectParser::from_dialect(TaggedDialect::from_raw_dialect(
                crate::dialect::sqlite(),
            )),
        }
    }

    /// Create a parser with custom configuration.
    pub fn with_config(config: &ParserConfig) -> Self {
        Parser {
            inner: DialectParser::with_config(
                TaggedDialect::from_raw_dialect(crate::dialect::sqlite()),
                config,
            ),
        }
    }

    /// Bind source text and return a [`StatementCursor`] for iterating typed statements.
    pub fn parse(&self, source: &str) -> StatementCursor {
        StatementCursor {
            inner: self.inner.parse(source),
        }
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

// ── StatementCursor ──────────────────────────────────────────────────────

/// A streaming cursor over parsed SQL statements, yielding typed SQLite AST nodes.
pub struct StatementCursor {
    inner: DialectStatementCursor<'static, SqliteNodeFamily>,
}

impl StatementCursor {
    /// Parse the next SQL statement and return a typed AST node.
    ///
    /// Returns:
    /// - `Some(Ok(node))` — successfully parsed statement.
    /// - `Some(Err(e))` — syntax error; call again to continue with subsequent statements.
    /// - `None` — all input has been consumed.
    ///
    /// The returned node borrows from the cursor. Use `while let` to iterate:
    /// ```ignore
    /// while let Some(result) = cursor.next_statement() {
    ///     let stmt = result?;
    /// }
    /// ```
    pub fn next_statement(&mut self) -> Option<Result<crate::ast::Stmt<'_>, ParseError>> {
        self.inner.next_statement()
    }

    /// The source text bound to this cursor.
    pub fn source(&self) -> &str {
        self.inner.source()
    }

    /// Resolve a typed node ID back into a view struct.
    ///
    /// Returns `Some(node)` if the ID refers to a valid arena node of the
    /// correct type, or `None` if the ID is null, invalid, or mismatched.
    pub fn resolve<I: syntaqlite_parser::NodeId>(&self, id: I) -> Option<I::Node<'_>> {
        self.inner.resolve(id)
    }

    /// Wrap a [`RawNodeId`] into a [`NodeRef`] using this cursor's reader and dialect.
    pub fn node_ref(&self, id: RawNodeId) -> NodeRef<'_> {
        self.inner.node_ref(id)
    }
}

// ── Token ────────────────────────────────────────────────────────────────

/// A typed SQL token with kind and source text.
#[derive(Debug, Clone, Copy)]
pub struct Token<'a> {
    /// The SQLite token kind.
    pub kind: crate::TokenType,
    /// The text of the token (a slice of the source).
    pub text: &'a str,
}

// ── Tokenizer ────────────────────────────────────────────────────────────

/// A SQL tokenizer for the built-in SQLite dialect.
pub struct Tokenizer {
    inner: DialectTokenizer<'static, SqliteNodeFamily>,
}

impl Tokenizer {
    /// Create a tokenizer for the built-in SQLite dialect.
    pub fn new() -> Self {
        Tokenizer {
            inner: DialectTokenizer::from_dialect(TaggedDialect::from_raw_dialect(
                crate::dialect::sqlite(),
            )),
        }
    }

    /// Bind source text and return a [`TokenCursor`] for iterating typed tokens.
    pub fn tokenize<'a>(&self, source: &'a str) -> TokenCursor<'a> {
        TokenCursor {
            inner: self.inner.tokenize(source),
        }
    }

    /// Zero-copy variant: bind a null-terminated source.
    pub fn tokenize_cstr<'a>(&self, source: &'a std::ffi::CStr) -> TokenCursor<'a> {
        TokenCursor {
            inner: self.inner.tokenize_cstr(source),
        }
    }
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::new()
    }
}

// ── TokenCursor ──────────────────────────────────────────────────────────

/// A cursor yielding typed SQLite tokens.
pub struct TokenCursor<'a> {
    inner: DialectTokenCursor<'a, SqliteNodeFamily>,
}

impl<'a> Iterator for TokenCursor<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|t| Token {
            kind: t.kind,
            text: t.text,
        })
    }
}

// ── IncrementalParser ────────────────────────────────────────────────────

/// An incremental SQL parser for the built-in SQLite dialect.
///
/// Feeds tokens one at a time via [`IncrementalCursor`], yielding typed
/// AST nodes.
pub struct IncrementalParser {
    inner: DialectIncrementalParser<'static, SqliteNodeFamily>,
}

impl IncrementalParser {
    /// Create an incremental parser for the built-in SQLite dialect.
    pub fn new() -> Self {
        IncrementalParser {
            inner: DialectIncrementalParser::from_dialect(TaggedDialect::from_raw_dialect(
                crate::dialect::sqlite(),
            )),
        }
    }

    /// Bind source text and return an [`IncrementalCursor`] for token feeding.
    pub fn feed(&self, source: &str) -> IncrementalCursor {
        IncrementalCursor {
            inner: self.inner.feed(source),
        }
    }
}

impl Default for IncrementalParser {
    fn default() -> Self {
        Self::new()
    }
}

// ── IncrementalCursor ────────────────────────────────────────────────────

/// A cursor for token-by-token incremental parsing of SQLite SQL.
///
/// Feed tokens via [`feed_token`](Self::feed_token) and signal end-of-input
/// via [`finish`](Self::finish).
pub struct IncrementalCursor {
    inner: DialectIncrementalCursor<'static, SqliteNodeFamily>,
}

impl IncrementalCursor {
    /// Feed a typed token to the parser.
    ///
    /// Returns `Ok(Some(node))` when a statement completes, `Ok(None)` to
    /// keep going, or `Err` on parse error. The returned node borrows from
    /// the cursor.
    pub fn feed_token(
        &mut self,
        token_type: crate::TokenType,
        span: Range<usize>,
    ) -> Result<Option<crate::ast::Stmt<'_>>, ParseError> {
        self.inner.feed_token(token_type, span)
    }

    /// Signal end of input.
    ///
    /// Returns `Ok(Some(node))` if a final statement completed, `Ok(None)`
    /// if there was nothing pending, or `Err` on parse error.
    ///
    /// No further methods may be called after `finish()`.
    pub fn finish(&mut self) -> Result<Option<crate::ast::Stmt<'_>>, ParseError> {
        self.inner.finish()
    }

    /// Return the [`NodeRef`] for the last completed statement.
    pub fn root(&self) -> Option<NodeRef<'_>> {
        self.inner.root()
    }

    /// Return the number of nodes currently in the parser arena.
    pub fn node_count(&self) -> u32 {
        self.inner.node_count()
    }

    /// Mark subsequent fed tokens as inside a macro expansion.
    pub fn begin_macro(&mut self, call_offset: u32, call_length: u32) {
        self.inner.begin_macro(call_offset, call_length)
    }

    /// End the innermost macro expansion region.
    pub fn end_macro(&mut self) {
        self.inner.end_macro()
    }

    /// Return all comments captured during parsing.
    pub fn comments(&self) -> &[Comment] {
        self.inner.comments()
    }

    /// Return all macro regions recorded via `begin_macro`/`end_macro`.
    pub fn macro_regions(&self) -> &[MacroRegion] {
        self.inner.macro_regions()
    }

    /// Resolve a typed node ID back into a view struct.
    ///
    /// Returns `Some(node)` if the ID refers to a valid arena node of the
    /// correct type, or `None` if the ID is null, invalid, or mismatched.
    pub fn resolve<I: syntaqlite_parser::NodeId>(&self, id: I) -> Option<I::Node<'_>> {
        self.inner.resolve(id)
    }
}

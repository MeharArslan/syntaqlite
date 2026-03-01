// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Typed SQLite wrappers around the dialect-agnostic base parser and tokenizer.
//!
//! These types provide an ergonomic API for the SQLite dialect: `Parser::new()`
//! creates a parser whose cursors yield typed [`Stmt`](crate::sqlite::ast::Stmt)
//! nodes directly, and `Tokenizer::new()` yields typed
//! [`Token`]s with [`TokenType`](crate::sqlite::low_level::TokenType) variants.
//!
//! For dialect-agnostic or custom-dialect usage, use the [`BaseParser`] and
//! [`BaseTokenizer`] types directly.

use super::session::{BaseParser, BaseParserBuilder, BaseStatementCursor, CursorBase, NodeReader, ParseError};
use super::tokenizer::{BaseTokenCursor, BaseTokenizer};
use super::nodes::NodeId;

// ── Parser ──────────────────────────────────────────────────────────────

/// A SQL parser for the built-in SQLite dialect.
///
/// Wraps [`BaseParser`] and yields typed [`Stmt`](crate::sqlite::ast::Stmt)
/// nodes from the parser arena.
///
/// # Example
///
/// ```
/// use syntaqlite::Parser;
///
/// let mut parser = Parser::new();
/// let mut cursor = parser.parse("SELECT 1");
/// let stmt = cursor.next_statement().unwrap().unwrap();
/// ```
pub struct Parser {
    inner: BaseParser<'static>,
}

// SAFETY: BaseParser is Send, and Parser is a thin wrapper.
unsafe impl Send for Parser {}

impl Parser {
    /// Create a parser for the built-in SQLite dialect with default configuration.
    pub fn new() -> Self {
        Parser {
            inner: BaseParser::new(),
        }
    }

    /// Create a builder for configuring the parser before construction.
    pub fn builder() -> ParserBuilder {
        ParserBuilder {
            inner: BaseParser::builder(&crate::sqlite::DIALECT),
        }
    }

    /// Bind source text and return a [`StatementCursor`] for iterating typed
    /// statements.
    pub fn parse<'a>(&'a mut self, source: &'a str) -> StatementCursor<'a> {
        StatementCursor {
            inner: self.inner.parse(source),
        }
    }

    /// Zero-copy variant: bind a null-terminated source and return a
    /// [`StatementCursor`].
    pub fn parse_cstr<'a>(&'a mut self, source: &'a std::ffi::CStr) -> StatementCursor<'a> {
        StatementCursor {
            inner: self.inner.parse_cstr(source),
        }
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

// ── ParserBuilder ───────────────────────────────────────────────────────

/// Builder for configuring a [`Parser`] before construction.
pub struct ParserBuilder {
    inner: BaseParserBuilder<'static>,
}

impl ParserBuilder {
    /// Enable parser trace output (Lemon debug trace).
    pub fn trace(mut self, enable: bool) -> Self {
        self.inner = self.inner.trace(enable);
        self
    }

    /// Set dialect config for version/cflag-gated tokenization.
    pub fn dialect_config(mut self, config: crate::dialect::ffi::DialectConfig) -> Self {
        self.inner = self.inner.dialect_config(config);
        self
    }

    /// Build the parser.
    pub fn build(self) -> Parser {
        Parser {
            inner: self.inner.build(),
        }
    }
}

// ── StatementCursor ─────────────────────────────────────────────────────

/// A streaming cursor over parsed SQL statements, yielding typed
/// [`Stmt`](crate::sqlite::ast::Stmt) nodes.
pub struct StatementCursor<'a> {
    inner: BaseStatementCursor<'a>,
}

impl<'a> StatementCursor<'a> {
    /// Parse the next SQL statement and return a typed AST node.
    ///
    /// Returns:
    /// - `Some(Ok(stmt))` — successfully parsed and resolved statement.
    /// - `Some(Err(e))` — syntax error; call again to continue with
    ///   subsequent statements.
    /// - `None` — all input has been consumed.
    pub fn next_statement(&mut self) -> Option<Result<crate::sqlite::ast::Stmt<'a>, ParseError>> {
        self.inner.next_statement().map(|result| {
            result.and_then(|node_ref| {
                let node_id = node_ref.id();
                node_ref.as_typed().ok_or_else(|| ParseError {
                    message: "failed to resolve typed AST node".to_string(),
                    offset: None,
                    length: None,
                    root: Some(node_id),
                })
            })
        })
    }

    /// Get a reference to the embedded [`NodeReader`].
    pub fn reader(&self) -> &NodeReader<'a> {
        self.inner.reader()
    }

    /// The source text bound to this cursor.
    pub fn source(&self) -> &'a str {
        self.inner.source()
    }

    /// Access the underlying cursor base for read-only operations
    /// (tokens, comments, macro regions).
    pub fn base(&self) -> &CursorBase<'a> {
        self.inner.base()
    }

    /// Dump an AST node tree as indented text.
    pub fn dump_node(&self, id: NodeId, out: &mut String, indent: usize) {
        self.inner.dump_node(id, out, indent)
    }
}

impl<'a> Iterator for StatementCursor<'a> {
    type Item = Result<crate::sqlite::ast::Stmt<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_statement()
    }
}

// ── Token ───────────────────────────────────────────────────────────────

/// A typed token with a [`TokenType`](crate::sqlite::low_level::TokenType)
/// variant and the source text slice.
#[derive(Debug, Clone, Copy)]
pub struct Token<'a> {
    /// The typed token kind.
    pub kind: crate::sqlite::low_level::TokenType,
    /// The text of the token (a slice of the source).
    pub text: &'a str,
}

// ── Tokenizer ───────────────────────────────────────────────────────────

/// A tokenizer for the built-in SQLite dialect, yielding typed [`Token`]s.
///
/// # Example
///
/// ```
/// use syntaqlite::Tokenizer;
///
/// let mut tokenizer = Tokenizer::new();
/// for token in tokenizer.tokenize("SELECT 1") {
///     println!("{:?}: {:?}", token.kind, token.text);
/// }
/// ```
pub struct Tokenizer {
    inner: BaseTokenizer,
}

// SAFETY: BaseTokenizer is Send, and Tokenizer is a thin wrapper.
unsafe impl Send for Tokenizer {}

impl Tokenizer {
    /// Create a tokenizer for the built-in SQLite dialect.
    pub fn new() -> Self {
        Tokenizer {
            inner: BaseTokenizer::new(),
        }
    }

    /// Create a builder for configuring the tokenizer before construction.
    pub fn builder() -> TokenizerBuilder {
        TokenizerBuilder {
            inner: BaseTokenizer::builder(*crate::sqlite::DIALECT),
        }
    }

    /// Bind source text and return a [`TokenCursor`] for iterating typed tokens.
    pub fn tokenize<'a>(&'a mut self, source: &'a str) -> TokenCursor<'a> {
        TokenCursor {
            inner: self.inner.tokenize(source),
        }
    }

    /// Zero-copy variant: bind a null-terminated source and return a
    /// [`TokenCursor`].
    pub fn tokenize_cstr<'a>(&'a mut self, source: &'a std::ffi::CStr) -> TokenCursor<'a> {
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

// ── TokenizerBuilder ────────────────────────────────────────────────────

/// Builder for configuring a [`Tokenizer`] before construction.
pub struct TokenizerBuilder {
    inner: crate::parser::tokenizer::BaseTokenizerBuilder<'static>,
}

impl TokenizerBuilder {
    /// Set dialect config for version/cflag-gated tokenization.
    pub fn dialect_config(mut self, config: crate::dialect::ffi::DialectConfig) -> Self {
        self.inner = self.inner.dialect_config(config);
        self
    }

    /// Build the tokenizer.
    pub fn build(self) -> Tokenizer {
        Tokenizer {
            inner: self.inner.build(),
        }
    }
}

// ── TokenCursor ─────────────────────────────────────────────────────────

/// An active tokenizer cursor yielding typed [`Token`]s.
pub struct TokenCursor<'a> {
    inner: BaseTokenCursor<'a>,
}

impl<'a> Iterator for TokenCursor<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let raw = self.inner.next()?;
            if let Ok(kind) =
                crate::sqlite::low_level::TokenType::try_from(raw.token_type)
            {
                return Some(Token {
                    kind,
                    text: raw.text,
                });
            }
            // Skip tokens with unknown type ordinals (shouldn't happen
            // with a well-formed dialect, but be defensive).
        }
    }
}

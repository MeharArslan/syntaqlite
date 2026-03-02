// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Typed wrappers parameterized over [`NodeFamily`], providing clean
//! type-inferred construction from a tagged [`Dialect`].

use std::marker::PhantomData;
use std::ops::Range;

use syntaqlite_parser::{
    Comment, Dialect, DialectConfig, MacroRegion, NodeFamily, NodeRef, ParseError,
    RawIncrementalCursor, RawIncrementalParser, RawIncrementalParserBuilder, RawNodeReader,
    RawParser, RawParserBuilder, RawStatementCursor, RawTokenCursor, RawTokenizer,
    RawTokenizerBuilder,
};

// ── Parser ───────────────────────────────────────────────────────────────

/// A SQL parser bound to a specific dialect.
///
/// Constructed from a tagged [`Dialect<'d, N>`] so that `N` (the node
/// family) is inferred automatically.
pub struct Parser<'d, N: NodeFamily> {
    inner: RawParser<'d>,
    _marker: PhantomData<N>,
}

#[cfg(feature = "sqlite")]
impl Parser<'static, syntaqlite_parser_sqlite::SqliteNodeFamily> {
    /// Create a parser for the built-in SQLite dialect with default configuration.
    pub fn new() -> Self {
        let dialect = Dialect::from_raw_dialect(crate::dialect::sqlite());
        Self::builder(dialect).build()
    }
}

#[cfg(feature = "sqlite")]
impl Default for Parser<'static, syntaqlite_parser_sqlite::SqliteNodeFamily> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'d, N: NodeFamily> Parser<'d, N> {
    /// Create a builder for a parser bound to the given dialect.
    pub fn builder(dialect: Dialect<'d, N>) -> ParserBuilder<'d, N> {
        ParserBuilder {
            inner: RawParser::builder(dialect.raw()),
            _marker: PhantomData,
        }
    }

    /// Bind source text and return a [`StatementCursor`] for iterating typed statements.
    pub fn parse<'a>(&'a mut self, source: &'a str) -> StatementCursor<'a, N> {
        StatementCursor {
            inner: self.inner.parse(source),
            _marker: PhantomData,
        }
    }

    /// Zero-copy variant: bind a null-terminated source.
    pub fn parse_cstr<'a>(&'a mut self, source: &'a std::ffi::CStr) -> StatementCursor<'a, N> {
        StatementCursor {
            inner: self.inner.parse_cstr(source),
            _marker: PhantomData,
        }
    }
}

// ── ParserBuilder ────────────────────────────────────────────────────────

/// Builder for [`Parser`].
pub struct ParserBuilder<'d, N: NodeFamily> {
    inner: RawParserBuilder<'d>,
    _marker: PhantomData<N>,
}

impl<'d, N: NodeFamily> ParserBuilder<'d, N> {
    /// Enable parser trace output.
    pub fn trace(mut self, enable: bool) -> Self {
        self.inner = self.inner.trace(enable);
        self
    }

    /// Collect token positions during parsing.
    pub fn collect_tokens(mut self, enable: bool) -> Self {
        self.inner = self.inner.collect_tokens(enable);
        self
    }

    /// Set dialect config for version/cflag-gated parsing.
    pub fn dialect_config(mut self, config: DialectConfig) -> Self {
        self.inner = self.inner.dialect_config(config);
        self
    }

    /// Build the parser.
    pub fn build(self) -> Parser<'d, N> {
        Parser {
            inner: self.inner.build(),
            _marker: PhantomData,
        }
    }
}

// ── StatementCursor ──────────────────────────────────────────────────────

/// A streaming cursor over parsed SQL statements, yielding typed nodes.
pub struct StatementCursor<'a, N: NodeFamily> {
    inner: RawStatementCursor<'a>,
    _marker: PhantomData<N>,
}

impl<'a, N: NodeFamily> StatementCursor<'a, N> {
    /// Parse the next SQL statement and return a typed AST node.
    ///
    /// Returns:
    /// - `Some(Ok(node))` — successfully parsed statement.
    /// - `Some(Err(e))` — syntax error; call again to continue with subsequent statements.
    /// - `None` — all input has been consumed.
    pub fn next_statement(&mut self) -> Option<Result<N::Node<'a>, ParseError>> {
        self.inner.next_statement().map(|result| {
            result.and_then(|node_ref| {
                let id = node_ref.id();
                node_ref
                    .as_typed::<N::Node<'a>>()
                    .ok_or_else(|| ParseError {
                        message: "failed to resolve typed AST node".to_string(),
                        offset: None,
                        length: None,
                        root: Some(id),
                    })
            })
        })
    }

    /// Get a reference to the embedded node reader.
    pub fn reader(&self) -> RawNodeReader<'a> {
        self.inner.reader()
    }

    /// The source text bound to this cursor.
    pub fn source(&self) -> &'a str {
        self.inner.source()
    }
}

impl<'a, N: NodeFamily> Iterator for StatementCursor<'a, N> {
    type Item = Result<N::Node<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_statement()
    }
}

// ── Token ────────────────────────────────────────────────────────────────

/// A typed SQL token with kind and source text.
#[derive(Debug, Clone, Copy)]
pub struct Token<'a, N: NodeFamily> {
    /// The typed token kind.
    pub kind: N::Token,
    /// The text of the token (a slice of the source).
    pub text: &'a str,
}

// ── Tokenizer ────────────────────────────────────────────────────────────

/// A SQL tokenizer bound to a specific dialect.
pub struct Tokenizer<'d, N: NodeFamily> {
    inner: RawTokenizer<'d>,
    _marker: PhantomData<N>,
}

#[cfg(feature = "sqlite")]
impl Tokenizer<'static, syntaqlite_parser_sqlite::SqliteNodeFamily> {
    /// Create a tokenizer for the built-in SQLite dialect with default configuration.
    pub fn new() -> Self {
        let dialect = Dialect::from_raw_dialect(crate::dialect::sqlite());
        Self::builder(dialect).build()
    }
}

#[cfg(feature = "sqlite")]
impl Default for Tokenizer<'static, syntaqlite_parser_sqlite::SqliteNodeFamily> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'d, N: NodeFamily> Tokenizer<'d, N> {
    /// Create a builder for a tokenizer bound to the given dialect.
    pub fn builder(dialect: Dialect<'d, N>) -> TokenizerBuilder<'d, N> {
        TokenizerBuilder {
            inner: RawTokenizer::builder(dialect.raw()),
            _marker: PhantomData,
        }
    }

    /// Bind source text and return a [`TokenCursor`] for iterating typed tokens.
    pub fn tokenize<'a>(&'a mut self, source: &'a str) -> TokenCursor<'a, N> {
        TokenCursor {
            inner: self.inner.tokenize(source),
            _marker: PhantomData,
        }
    }

    /// Zero-copy variant: bind a null-terminated source.
    pub fn tokenize_cstr<'a>(&'a mut self, source: &'a std::ffi::CStr) -> TokenCursor<'a, N> {
        TokenCursor {
            inner: self.inner.tokenize_cstr(source),
            _marker: PhantomData,
        }
    }
}

// ── TokenizerBuilder ─────────────────────────────────────────────────────

/// Builder for [`Tokenizer`].
pub struct TokenizerBuilder<'d, N: NodeFamily> {
    inner: RawTokenizerBuilder<'d>,
    _marker: PhantomData<N>,
}

impl<'d, N: NodeFamily> TokenizerBuilder<'d, N> {
    /// Set dialect config for version/cflag-gated tokenization.
    pub fn dialect_config(mut self, config: DialectConfig) -> Self {
        self.inner = self.inner.dialect_config(config);
        self
    }

    /// Build the tokenizer.
    pub fn build(self) -> Tokenizer<'d, N> {
        Tokenizer {
            inner: self.inner.build(),
            _marker: PhantomData,
        }
    }
}

// ── TokenCursor ──────────────────────────────────────────────────────────

/// A cursor yielding typed tokens.
pub struct TokenCursor<'a, N: NodeFamily> {
    inner: RawTokenCursor<'a>,
    _marker: PhantomData<N>,
}

impl<'a, N: NodeFamily> Iterator for TokenCursor<'a, N> {
    type Item = Token<'a, N>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let raw = self.inner.next()?;
            if let Some(kind) =
                <N::Token as syntaqlite_parser::DialectTokenType>::from_token_type(raw.token_type)
            {
                return Some(Token {
                    kind,
                    text: raw.text,
                });
            }
            // Skip tokens with unknown type ordinals.
        }
    }
}

// ── IncrementalParser ────────────────────────────────────────────────────

/// An incremental SQL parser bound to a specific dialect.
///
/// Feeds tokens one at a time via [`IncrementalCursor`], yielding typed
/// AST nodes.
pub struct IncrementalParser<'d, N: NodeFamily> {
    inner: RawIncrementalParser<'d>,
    _marker: PhantomData<N>,
}

#[cfg(feature = "sqlite")]
impl IncrementalParser<'static, syntaqlite_parser_sqlite::SqliteNodeFamily> {
    /// Create an incremental parser for the built-in SQLite dialect.
    pub fn new() -> Self {
        let dialect = Dialect::from_raw_dialect(crate::dialect::sqlite());
        Self::builder(dialect).build()
    }
}

#[cfg(feature = "sqlite")]
impl Default for IncrementalParser<'static, syntaqlite_parser_sqlite::SqliteNodeFamily> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'d, N: NodeFamily> IncrementalParser<'d, N> {
    /// Create a builder for an incremental parser bound to the given dialect.
    pub fn builder(dialect: Dialect<'d, N>) -> IncrementalParserBuilder<'d, N> {
        IncrementalParserBuilder {
            inner: RawIncrementalParser::builder(dialect.raw()),
            _marker: PhantomData,
        }
    }

    /// Bind source text and return an [`IncrementalCursor`] for token feeding.
    pub fn feed<'a>(&'a mut self, source: &'a str) -> IncrementalCursor<'a, N> {
        IncrementalCursor {
            inner: self.inner.feed(source),
            last_root: None,
            _marker: PhantomData,
        }
    }

    /// Zero-copy variant: bind a null-terminated source.
    pub fn feed_cstr<'a>(&'a mut self, source: &'a std::ffi::CStr) -> IncrementalCursor<'a, N> {
        IncrementalCursor {
            inner: self.inner.feed_cstr(source),
            last_root: None,
            _marker: PhantomData,
        }
    }
}

// ── IncrementalParserBuilder ─────────────────────────────────────────────

/// Builder for [`IncrementalParser`].
pub struct IncrementalParserBuilder<'d, N: NodeFamily> {
    inner: RawIncrementalParserBuilder<'d>,
    _marker: PhantomData<N>,
}

impl<'d, N: NodeFamily> IncrementalParserBuilder<'d, N> {
    /// Enable parser trace output.
    pub fn trace(mut self, enable: bool) -> Self {
        self.inner = self.inner.trace(enable);
        self
    }

    /// Collect non-whitespace token positions during parsing.
    pub fn collect_tokens(mut self, enable: bool) -> Self {
        self.inner = self.inner.collect_tokens(enable);
        self
    }

    /// Set dialect config for version/cflag-gated parsing.
    pub fn dialect_config(mut self, config: DialectConfig) -> Self {
        self.inner = self.inner.dialect_config(config);
        self
    }

    /// Build the incremental parser.
    pub fn build(self) -> IncrementalParser<'d, N> {
        IncrementalParser {
            inner: self.inner.build(),
            _marker: PhantomData,
        }
    }
}

// ── IncrementalCursor ────────────────────────────────────────────────────

/// A cursor for token-by-token incremental parsing.
///
/// Feed tokens via [`feed_token`](Self::feed_token) and signal end-of-input
/// via [`finish`](Self::finish).
pub struct IncrementalCursor<'a, N: NodeFamily> {
    inner: RawIncrementalCursor<'a>,
    last_root: Option<NodeRef<'a>>,
    _marker: PhantomData<N>,
}

impl<'a, N: NodeFamily> IncrementalCursor<'a, N> {
    /// Feed a typed token to the parser.
    ///
    /// Returns `Ok(Some(node))` when a statement completes, `Ok(None)` to
    /// keep going, or `Err` on parse error.
    pub fn feed_token(
        &mut self,
        token_type: N::Token,
        span: Range<usize>,
    ) -> Result<Option<N::Node<'a>>, ParseError> {
        match self.inner.feed_token(token_type.into(), span)? {
            None => Ok(None),
            Some(id) => {
                let node_ref = self.inner.node_ref(id);
                self.last_root = Some(node_ref);
                let node = node_ref
                    .as_typed::<N::Node<'a>>()
                    .ok_or_else(|| ParseError {
                        message: "failed to resolve typed AST node".to_string(),
                        offset: None,
                        length: None,
                        root: Some(id),
                    })?;
                Ok(Some(node))
            }
        }
    }

    /// Signal end of input.
    ///
    /// Returns `Ok(Some(node))` if a final statement completed, `Ok(None)`
    /// if there was nothing pending, or `Err` on parse error.
    ///
    /// No further methods may be called after `finish()`.
    pub fn finish(&mut self) -> Result<Option<N::Node<'a>>, ParseError> {
        match self.inner.finish()? {
            None => Ok(None),
            Some(id) => {
                let node_ref = self.inner.node_ref(id);
                self.last_root = Some(node_ref);
                let node = node_ref
                    .as_typed::<N::Node<'a>>()
                    .ok_or_else(|| ParseError {
                        message: "failed to resolve typed AST node".to_string(),
                        offset: None,
                        length: None,
                        root: Some(id),
                    })?;
                Ok(Some(node))
            }
        }
    }

    /// Return the [`NodeRef`] for the last completed statement.
    pub fn root(&self) -> Option<NodeRef<'a>> {
        self.last_root
    }

    /// Return terminal token IDs valid at the current parser state, as raw u32 ordinals.
    pub fn expected_tokens(&self) -> Vec<u32> {
        self.inner.expected_tokens()
    }

    /// Return the semantic completion context at the current parser state.
    pub fn completion_context(&self) -> u32 {
        self.inner.completion_context()
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

    /// Get the embedded node reader.
    pub fn reader(&self) -> RawNodeReader<'a> {
        self.inner.reader()
    }

    /// Return all comments captured during parsing.
    pub fn comments(&self) -> &[Comment] {
        self.inner.comments()
    }

    /// Return all macro regions recorded via `begin_macro`/`end_macro`.
    pub fn macro_regions(&self) -> &[MacroRegion] {
        self.inner.macro_regions()
    }
}

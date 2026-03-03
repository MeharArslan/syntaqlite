// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Dialect-generic typed wrappers parameterized over [`NodeFamily`].
//!
//! These types provide clean type-inferred construction from a tagged
//! [`Dialect`].  For the built-in SQLite dialect, use the concrete
//! wrappers at the crate root ([`crate::Parser`], [`crate::Tokenizer`],
//! etc.) instead.

use std::marker::PhantomData;
use std::ops::Range;

use syntaqlite_parser::{
    Comment, Dialect, DialectNodeType, MacroRegion, NodeFamily, NodeRef, ParseError, ParserConfig,
    RawIncrementalCursor, RawIncrementalParser, RawParser, RawStatementCursor, RawTokenCursor,
    RawTokenizer,
};

// ── DialectParser ───────────────────────────────────────────────────────

/// A SQL parser bound to a specific dialect.
///
/// Constructed from a tagged [`Dialect<'d, N>`] so that `N` (the node
/// family) is inferred automatically.
pub struct DialectParser<'d, N: NodeFamily> {
    inner: RawParser<'d>,
    _marker: PhantomData<N>,
}

impl<'d, N: NodeFamily> DialectParser<'d, N> {
    /// Create a parser bound to the given dialect with default configuration.
    pub fn from_dialect(dialect: Dialect<'d, N>) -> Self {
        DialectParser {
            inner: RawParser::new(dialect.raw()),
            _marker: PhantomData,
        }
    }

    /// Create a parser bound to the given dialect with custom configuration.
    pub fn with_config(dialect: Dialect<'d, N>, config: &ParserConfig) -> Self {
        DialectParser {
            inner: RawParser::with_config(dialect.raw(), config),
            _marker: PhantomData,
        }
    }

    /// Bind source text and return a [`DialectStatementCursor`] for iterating typed statements.
    pub fn parse<'a>(&self, source: &'a str) -> DialectStatementCursor<'a, N>
    where
        'd: 'a,
    {
        DialectStatementCursor {
            inner: self.inner.parse(source),
            _marker: PhantomData,
        }
    }

    /// Zero-copy variant: bind a null-terminated source.
    pub fn parse_cstr<'a>(&self, source: &'a std::ffi::CStr) -> DialectStatementCursor<'a, N>
    where
        'd: 'a,
    {
        DialectStatementCursor {
            inner: self.inner.parse_cstr(source),
            _marker: PhantomData,
        }
    }
}

// ── DialectStatementCursor ──────────────────────────────────────────────

/// A streaming cursor over parsed SQL statements, yielding typed nodes.
pub struct DialectStatementCursor<'a, N: NodeFamily> {
    inner: RawStatementCursor<'a>,
    _marker: PhantomData<N>,
}

impl<'a, N: NodeFamily> DialectStatementCursor<'a, N> {
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

    /// The source text bound to this cursor.
    pub fn source(&self) -> &'a str {
        self.inner.source()
    }

    /// Resolve a typed node ID back into a view struct.
    ///
    /// Returns `Some(node)` if the ID refers to a valid arena node of the
    /// correct type, or `None` if the ID is null, invalid, or mismatched.
    pub fn resolve<I: syntaqlite_parser::NodeId>(&self, id: I) -> Option<I::Node<'a>> {
        I::Node::from_arena(self.inner.reader(), id.into())
    }
}

impl<'a, N: NodeFamily> Iterator for DialectStatementCursor<'a, N> {
    type Item = Result<N::Node<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_statement()
    }
}

// ── DialectToken ────────────────────────────────────────────────────────

/// A typed SQL token with kind and source text.
#[derive(Debug, Clone, Copy)]
pub struct DialectToken<'a, N: NodeFamily> {
    /// The typed token kind.
    pub kind: N::Token,
    /// The text of the token (a slice of the source).
    pub text: &'a str,
}

// ── DialectTokenizer ────────────────────────────────────────────────────

/// A SQL tokenizer bound to a specific dialect.
pub struct DialectTokenizer<'d, N: NodeFamily> {
    inner: RawTokenizer<'d>,
    _marker: PhantomData<N>,
}

impl<'d, N: NodeFamily> DialectTokenizer<'d, N> {
    /// Create a tokenizer bound to the given dialect with default configuration.
    pub fn from_dialect(dialect: Dialect<'d, N>) -> Self {
        DialectTokenizer {
            inner: RawTokenizer::new(dialect.raw()),
            _marker: PhantomData,
        }
    }

    /// Bind source text and return a [`DialectTokenCursor`] for iterating typed tokens.
    pub fn tokenize<'a>(&self, source: &'a str) -> DialectTokenCursor<'a, N>
    where
        'd: 'a,
    {
        DialectTokenCursor {
            inner: self.inner.tokenize(source),
            _marker: PhantomData,
        }
    }

    /// Zero-copy variant: bind a null-terminated source.
    pub fn tokenize_cstr<'a>(&self, source: &'a std::ffi::CStr) -> DialectTokenCursor<'a, N>
    where
        'd: 'a,
    {
        DialectTokenCursor {
            inner: self.inner.tokenize_cstr(source),
            _marker: PhantomData,
        }
    }
}

// ── DialectTokenCursor ──────────────────────────────────────────────────

/// A cursor yielding typed tokens.
pub struct DialectTokenCursor<'a, N: NodeFamily> {
    inner: RawTokenCursor<'a>,
    _marker: PhantomData<N>,
}

impl<'a, N: NodeFamily> Iterator for DialectTokenCursor<'a, N> {
    type Item = DialectToken<'a, N>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let raw = self.inner.next()?;
            if let Some(kind) =
                <N::Token as syntaqlite_parser::DialectTokenType>::from_token_type(raw.token_type)
            {
                return Some(DialectToken {
                    kind,
                    text: raw.text,
                });
            }
            // Skip tokens with unknown type ordinals.
        }
    }
}

// ── DialectIncrementalParser ────────────────────────────────────────────

/// An incremental SQL parser bound to a specific dialect.
///
/// Feeds tokens one at a time via [`DialectIncrementalCursor`], yielding
/// typed AST nodes.
pub struct DialectIncrementalParser<'d, N: NodeFamily> {
    inner: RawIncrementalParser<'d>,
    _marker: PhantomData<N>,
}

impl<'d, N: NodeFamily> DialectIncrementalParser<'d, N> {
    /// Create an incremental parser bound to the given dialect with default
    /// configuration (token collection enabled).
    pub fn from_dialect(dialect: Dialect<'d, N>) -> Self {
        DialectIncrementalParser {
            inner: RawIncrementalParser::new(dialect.raw()),
            _marker: PhantomData,
        }
    }

    /// Create an incremental parser bound to the given dialect with custom
    /// configuration.
    pub fn with_config(dialect: Dialect<'d, N>, config: &ParserConfig) -> Self {
        DialectIncrementalParser {
            inner: RawIncrementalParser::with_config(dialect.raw(), config),
            _marker: PhantomData,
        }
    }

    /// Bind source text and return a [`DialectIncrementalCursor`] for token feeding.
    pub fn feed<'a>(&self, source: &'a str) -> DialectIncrementalCursor<'a, N>
    where
        'd: 'a,
    {
        DialectIncrementalCursor {
            inner: self.inner.feed(source),
            last_root: None,
            _marker: PhantomData,
        }
    }

    /// Zero-copy variant: bind a null-terminated source.
    pub fn feed_cstr<'a>(&self, source: &'a std::ffi::CStr) -> DialectIncrementalCursor<'a, N>
    where
        'd: 'a,
    {
        DialectIncrementalCursor {
            inner: self.inner.feed_cstr(source),
            last_root: None,
            _marker: PhantomData,
        }
    }
}

// ── DialectIncrementalCursor ────────────────────────────────────────────

/// A cursor for token-by-token incremental parsing.
///
/// Feed tokens via [`feed_token`](Self::feed_token) and signal end-of-input
/// via [`finish`](Self::finish).
pub struct DialectIncrementalCursor<'a, N: NodeFamily> {
    inner: RawIncrementalCursor<'a>,
    last_root: Option<NodeRef<'a>>,
    _marker: PhantomData<N>,
}

impl<'a, N: NodeFamily> DialectIncrementalCursor<'a, N> {
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
    pub fn resolve<I: syntaqlite_parser::NodeId>(&self, id: I) -> Option<I::Node<'a>> {
        I::Node::from_arena(self.inner.reader(), id.into())
    }
}

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Typed wrappers around the dialect-agnostic base parser and tokenizer.
//!
//! This module provides generic types parameterized over node types (via [`FromArena`])
//! and token types (via [`DialectTokenType`]), enabling any dialect to build ergonomic
//! typed APIs.
//!
//! Dialect crates should:
//! 1. Implement [`DialectTokenType`] for their token enum
//! 2. Define convenience type aliases over the generic [`TypedTokenizer<T>`] and [`TypedTokenCursor<'a, T>`]
//! 3. Use [`RawParser`] with [`TypedStatementCursor<'a, N>`] for nodes
//!
//! # Example for a custom dialect crate
//!
//! ```ignore
//! // In your dialect crate's token module:
//! impl DialectTokenType for MyTokenType {
//!     fn from_token_type(raw: u32) -> Option<Self> {
//!         MyTokenType::from_raw(raw)
//!     }
//! }
//!
//! // Convenience aliases in your public API:
//! pub type Tokenizer = crate::parser::typed::TypedTokenizer<MyTokenType>;
//! pub type TokenCursor<'a> = crate::parser::typed::TypedTokenCursor<'a, MyTokenType>;
//! pub type Token<'a> = crate::parser::typed::TypedToken<'a, MyTokenType>;
//! ```

use super::nodes::NodeId;
use super::session::{ParseError, RawNodeReader, RawParser, RawParserBuilder, RawStatementCursor};
use super::tokenizer::{RawTokenCursor, RawTokenizer};
use super::typed_list::FromArena;

// ── TypedParser ──────────────────────────────────────────────────────────

/// A generic parser wrapping [`RawParser`], pre-bound to a static dialect.
///
/// The node type `N` is chosen at the `parse()` call site, so a single
/// `TypedParser` instance can be shared across node types.
pub struct TypedParser {
    inner: RawParser<'static>,
}

// SAFETY: RawParser is Send; TypedParser is a thin wrapper.
unsafe impl Send for TypedParser {}

impl TypedParser {
    /// Create a parser bound to the given static dialect.
    pub fn new(dialect: &'static crate::dialect::Dialect<'static>) -> Self {
        Self::builder(dialect).build()
    }

    /// Create a builder for more detailed configuration.
    pub fn builder(dialect: &'static crate::dialect::Dialect<'static>) -> TypedParserBuilder {
        TypedParserBuilder {
            inner: RawParser::builder(dialect),
        }
    }

    /// Bind source text and return a [`TypedStatementCursor`].
    ///
    /// `N` is inferred from the return type (e.g. the dialect's `Stmt<'a>`).
    pub fn parse<'a, N: FromArena<'a>>(
        &'a mut self,
        source: &'a str,
    ) -> TypedStatementCursor<'a, N> {
        TypedStatementCursor::new(self.inner.parse(source))
    }

    /// Zero-copy variant: bind a null-terminated source.
    pub fn parse_cstr<'a, N: FromArena<'a>>(
        &'a mut self,
        source: &'a std::ffi::CStr,
    ) -> TypedStatementCursor<'a, N> {
        TypedStatementCursor::new(self.inner.parse_cstr(source))
    }
}

// ── TypedParserBuilder ────────────────────────────────────────────────────

/// Builder for [`TypedParser`].
pub struct TypedParserBuilder {
    inner: RawParserBuilder<'static>,
}

impl TypedParserBuilder {
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
    pub fn dialect_config(mut self, config: crate::dialect::ffi::DialectConfig) -> Self {
        self.inner = self.inner.dialect_config(config);
        self
    }

    /// Build the parser.
    pub fn build(self) -> TypedParser {
        TypedParser {
            inner: self.inner.build(),
        }
    }
}

// ── DialectTokenType trait ───────────────────────────────────────────────

/// A token type that can be resolved from a raw token integer.
///
/// Each dialect's token enum must implement this trait to enable
/// generic [`TypedTokenizer<T>`] and [`TypedTokenCursor<'a, T>`] usage.
///
/// # Example
///
/// ```ignore
/// impl DialectTokenType for MyTokenType {
///     fn from_token_type(raw: u32) -> Option<Self> {
///         MyTokenType::from_raw(raw)
///     }
/// }
/// ```
pub trait DialectTokenType: Sized + Clone + Copy + std::fmt::Debug {
    /// Attempt to resolve a raw token type code into this dialect's token variant.
    fn from_token_type(raw: u32) -> Option<Self>;
}

// ── TypedStatementCursor ────────────────────────────────────────────────

/// A generic streaming cursor over parsed SQL statements.
///
/// Yields nodes of type `N` (must implement [`FromArena`]) by resolving
/// [`NodeRef`](crate::raw::NodeRef) from the underlying [`RawStatementCursor`].
///
/// # Type parameter
/// - `N: FromArena<'a>` — the typed node type (generated by codegen for each dialect)
///
/// # Example
///
/// ```ignore
/// // For a dialect with a custom Node enum:
/// let mut cursor = TypedStatementCursor::<MyNode>::new(raw_cursor);
/// while let Some(result) = cursor.next_statement() {
///     let node = result?;
///     // Process MyNode...
/// }
/// ```
pub struct TypedStatementCursor<'a, N: FromArena<'a>> {
    inner: RawStatementCursor<'a>,
    _phantom: std::marker::PhantomData<N>,
}

impl<'a, N: FromArena<'a>> TypedStatementCursor<'a, N> {
    /// Create a typed cursor from a raw cursor.
    pub fn new(inner: RawStatementCursor<'a>) -> Self {
        TypedStatementCursor {
            inner,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Parse the next SQL statement and return a typed AST node.
    ///
    /// Returns:
    /// - `Some(Ok(node))` — successfully parsed and resolved statement.
    /// - `Some(Err(e))` — syntax error; call again to continue with subsequent statements.
    /// - `None` — all input has been consumed.
    pub fn next_statement(&mut self) -> Option<Result<N, ParseError>> {
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

    /// Get a reference to the embedded [`RawNodeReader`].
    pub fn reader(&self) -> &RawNodeReader<'a> {
        self.inner.reader()
    }

    /// The source text bound to this cursor.
    pub fn source(&self) -> &'a str {
        self.inner.source()
    }

    /// Dump an AST node tree as indented text.
    pub fn dump_node(&self, id: NodeId, out: &mut String, indent: usize) {
        self.inner.dump_node(id, out, indent)
    }
}

impl<'a, N: FromArena<'a>> Iterator for TypedStatementCursor<'a, N> {
    type Item = Result<N, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_statement()
    }
}

// ── TypedToken ──────────────────────────────────────────────────────────

/// A generic typed token with a dialect-specific token type variant and source text.
///
/// # Type parameter
/// - `T: DialectTokenType` — the dialect's token type enum (generated by codegen)
#[derive(Debug, Clone, Copy)]
pub struct TypedToken<'a, T: DialectTokenType> {
    /// The typed token kind.
    pub kind: T,
    /// The text of the token (a slice of the source).
    pub text: &'a str,
}

// ── TypedTokenizer ──────────────────────────────────────────────────────

/// A generic tokenizer for any dialect, yielding [`TypedToken`]s.
///
/// # Type parameter
/// - `T: DialectTokenType` — the dialect's token type (generated by codegen)
///
/// # Example
///
/// ```ignore
/// // For a custom dialect:
/// let mut tokenizer = TypedTokenizer::<MyTokenType>::new();
/// for token in tokenizer.tokenize("SELECT 1") {
///     println!("{:?}: {:?}", token.kind, token.text);
/// }
/// ```
pub struct TypedTokenizer<T: DialectTokenType> {
    inner: RawTokenizer,
    _phantom: std::marker::PhantomData<T>,
}

// SAFETY: RawTokenizer is Send, and TypedTokenizer is a thin wrapper.
unsafe impl<T: DialectTokenType> Send for TypedTokenizer<T> {}

impl<T: DialectTokenType> TypedTokenizer<T> {
    /// Create a tokenizer with default configuration.
    pub fn new(dialect: crate::dialect::Dialect<'static>) -> Self {
        Self::builder(dialect).build()
    }

    /// Create a builder for configuring the tokenizer before construction.
    pub fn builder(dialect: crate::dialect::Dialect<'static>) -> TypedTokenizerBuilder<T> {
        TypedTokenizerBuilder {
            inner: RawTokenizer::builder(dialect),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Bind source text and return a [`TypedTokenCursor`] for iterating typed tokens.
    pub fn tokenize<'a>(&'a mut self, source: &'a str) -> TypedTokenCursor<'a, T> {
        TypedTokenCursor {
            inner: self.inner.tokenize(source),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Zero-copy variant: bind a null-terminated source and return a [`TypedTokenCursor`].
    pub fn tokenize_cstr<'a>(&'a mut self, source: &'a std::ffi::CStr) -> TypedTokenCursor<'a, T> {
        TypedTokenCursor {
            inner: self.inner.tokenize_cstr(source),
            _phantom: std::marker::PhantomData,
        }
    }
}

// ── TypedTokenizerBuilder ────────────────────────────────────────────────

/// Builder for configuring a [`TypedTokenizer<T>`] before construction.
pub struct TypedTokenizerBuilder<T: DialectTokenType> {
    inner: crate::parser::tokenizer::RawTokenizerBuilder<'static>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: DialectTokenType> TypedTokenizerBuilder<T> {
    /// Set dialect config for version/cflag-gated tokenization.
    pub fn dialect_config(mut self, config: crate::dialect::ffi::DialectConfig) -> Self {
        self.inner = self.inner.dialect_config(config);
        self
    }

    /// Build the tokenizer.
    pub fn build(self) -> TypedTokenizer<T> {
        TypedTokenizer {
            inner: self.inner.build(),
            _phantom: std::marker::PhantomData,
        }
    }
}

// ── TypedTokenCursor ────────────────────────────────────────────────────

/// A generic cursor yielding [`TypedToken`]s of dialect-specific type `T`.
///
/// Skips tokens with unknown type ordinals (which shouldn't occur with
/// a well-formed dialect, but is handled defensively).
pub struct TypedTokenCursor<'a, T: DialectTokenType> {
    inner: RawTokenCursor<'a>,
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T: DialectTokenType> Iterator for TypedTokenCursor<'a, T> {
    type Item = TypedToken<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let raw = self.inner.next()?;
            if let Some(kind) = T::from_token_type(raw.token_type) {
                return Some(TypedToken {
                    kind,
                    text: raw.text,
                });
            }
            // Skip tokens with unknown type ordinals.
        }
    }
}

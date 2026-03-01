// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Fast, accurate SQL tooling for SQLite and its dialects.
//!
//! syntaqlite tokenizes, parses, formats, and validates SQLite SQL using
//! parser tables generated from SQLite's own Lemon grammar, so every
//! quirk of the real parser is faithfully reproduced.
//!
//! # Quick start
//!
//! **Parse** SQL into a typed AST:
//!
//! ```
//! use syntaqlite::Parser;
//!
//! let mut parser = Parser::new();
//! for stmt in parser.parse("SELECT 1 + 2; CREATE TABLE t(x)") {
//!     let stmt = stmt.expect("parse error");
//!     println!("{stmt:?}");
//! }
//! ```
//!
//! **Format** SQL:
//!
//! ```
//! use syntaqlite::Formatter;
//!
//! let mut fmt = Formatter::new();
//! let pretty = fmt.format("select a,b from t where x>1").unwrap();
//! assert_eq!(pretty, "SELECT a, b FROM t WHERE x > 1;\n");
//! ```
//!
//! **Validate** SQL against a schema:
//!
//! ```
//! use syntaqlite::Validator;
//! use syntaqlite::validation::ValidationConfig;
//!
//! let mut v = Validator::new();
//! let diags = v.validate("SELEC 1", None, &ValidationConfig::default());
//! assert!(!diags.is_empty());
//! ```
//!
//! # Feature flags
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `sqlite` | yes | SQLite dialect (grammar, tokens, typed AST, built-in functions) |
//! | `fmt` | yes | SQL formatter (bytecode interpreter) |
//! | `validation` | yes | Semantic validation (schema checks, fuzzy suggestions) |
//! | `embedded` | no | Embedded SQL extraction from Python/TypeScript host files |
//! | `lsp` | no | Language-server analysis host (semantic tokens, completions, formatting) |
//! | `pin-version` | no | Pin SQLite version at compile time for dead-code elimination |
//! | `pin-cflags` | no | Pin compile-time flags (`SQLITE_OMIT_*` / `SQLITE_ENABLE_*`) |
//!
//! # Crate layout
//!
//! The primary user-facing types — [`Parser`], [`Tokenizer`], [`Formatter`],
//! and [`Validator`] — are re-exported at the crate root and operate on the
//! built-in SQLite dialect.
//!
//! For lower-level or dialect-agnostic access, see:
//!
//! - [`sqlite`] — SQLite-specific typed AST ([`sqlite::ast`]), token types
//!   ([`sqlite::low_level`]), configuration, and built-in functions.
//! - [`dialect`] — The opaque [`Dialect`](dialect::Dialect) handle and
//!   semantic [`TokenCategory`](dialect::TokenCategory) enum.
//! - [`raw`] — Dialect-agnostic building blocks for external dialect crates
//!   (raw parsers, tokenizers, node types, and the [`DialectDef`](raw::DialectDef)
//!   trait).
//! - [`fmt`] — Formatter configuration ([`FormatConfig`](fmt::FormatConfig),
//!   [`KeywordCase`](fmt::KeywordCase)).
//! - [`validation`] — Validator configuration, diagnostic types, and
//!   schema context.
//! - [`embedded`] — Extract and validate SQL from Python f-strings and
//!   TypeScript template literals.
//! - [`lsp`] — [`AnalysisHost`](lsp::AnalysisHost) for editor integrations.

// Force-link the sys crate so the linker includes its native C libraries.
extern crate syntaqlite_parser_sys;

pub(crate) mod ast_traits;
pub mod parser;

// ── Top-level API ─────────────────────────────────────────────────────
//
// Only the 5 primary user-facing types are re-exported at the crate root.
// Everything else lives in its host module (parser::*, fmt::*, validation::*).

#[cfg(feature = "sqlite")]
pub use parser::typed::{Parser, Tokenizer};

pub use parser::token_parser::LowLevelParser as IncrementalParser;

// ── Formatter ────────────────────────────────────────────────────────────

#[cfg(feature = "fmt")]
pub mod fmt;
#[doc(inline)]
#[cfg(feature = "fmt")]
pub use fmt::formatter::Formatter;

// ── Dialect ──────────────────────────────────────────────────────────────

pub(crate) mod catalog;

pub mod dialect;

// ── Validation ───────────────────────────────────────────────────────────

#[cfg(feature = "validation")]
pub mod validation;
#[doc(inline)]
#[cfg(feature = "validation")]
pub use validation::Validator;

// ── Embedded SQL ─────────────────────────────────────────────────────────

#[cfg(feature = "embedded")]
pub mod embedded;

// ── LSP ──────────────────────────────────────────────────────────────────

#[cfg(feature = "lsp")]
pub mod lsp;

// ── SQLite dialect ───────────────────────────────────────────────────────

pub mod sqlite;

// ── Raw (dialect-agnostic) API ───────────────────────────────────────────
//
// Lower-level building blocks for external dialect crates and advanced use.

/// Dialect-agnostic ("raw") API for building custom dialect integrations.
///
/// Most users should prefer the top-level SQLite types ([`Parser`],
/// [`Formatter`], etc.). This module exposes the lower-level building
/// blocks that external dialect crates need.
pub mod raw {
    // ── Parser types ─────────────────────────────────────────────────────
    pub use crate::parser::session::BaseParser as RawParser;
    pub use crate::parser::session::BaseStatementCursor as RawStatementCursor;
    pub use crate::parser::session::NodeReader as RawNodeReader;
    pub use crate::parser::session::ErrorSpan;

    pub use crate::parser::tokenizer::BaseTokenizer as RawTokenizer;
    pub use crate::parser::tokenizer::BaseTokenCursor as RawTokenCursor;
    pub use crate::parser::tokenizer::RawToken;

    pub use crate::parser::token_parser::LowLevelParser as RawIncrementalParser;
    pub use crate::parser::token_parser::LowLevelCursor as RawIncrementalCursor;

    // ── Node / field types ───────────────────────────────────────────────
    pub use crate::parser::nodes::{ArenaNode, FieldVal, Fields, NodeId, NodeList, SourceSpan};
    pub use crate::parser::session::{NodeRef, ParseError};
    pub use crate::parser::typed_list::{FromArena, TypedList};

    // ── Token metadata ───────────────────────────────────────────────────
    pub use crate::parser::ffi::{
        Comment, CommentKind, TOKEN_FLAG_AS_FUNCTION, TOKEN_FLAG_AS_ID, TOKEN_FLAG_AS_TYPE,
    };

    // ── AST trait definitions ────────────────────────────────────────────
    pub use crate::ast_traits::*;

    // ── Builders ─────────────────────────────────────────────────────────

    /// Builder types for dialect-agnostic APIs.
    pub mod builders {
        pub use crate::parser::session::BaseParserBuilder as RawParserBuilder;
        pub use crate::parser::tokenizer::BaseTokenizerBuilder as RawTokenizerBuilder;
        pub use crate::parser::token_parser::LowLevelParserBuilder as RawIncrementalParserBuilder;

        #[cfg(feature = "fmt")]
        pub use crate::fmt::formatter::FormatterBuilder;

        #[cfg(feature = "validation")]
        pub use crate::validation::ValidatorBuilder;
    }

    // ── DialectDef trait + typed wrappers ────────────────────────────────

    pub use crate::dialect::Dialect;
    pub use crate::dialect::ffi::Dialect as FfiDialect;

    use std::marker::PhantomData;

    /// Trait implemented by each dialect to enable generic typed wrappers.
    ///
    /// Codegen produces an impl of this trait for each external dialect crate.
    /// SQLite does NOT use this — it has hand-written wrappers for a cleaner API.
    pub trait DialectDef {
        /// The static dialect handle.
        fn dialect() -> &'static Dialect<'static>;

        /// The typed statement enum (e.g. `mydb::ast::Stmt<'a>`).
        type Stmt<'a>: FromArena<'a>;

        /// The typed token enum (e.g. `mydb::tokens::TokenType`).
        type TokenType: Copy;

        /// Convert a raw token type ordinal to the typed enum.
        fn token_from_raw(raw: u32) -> Self::TokenType;
    }

    // ── TypedParser ──────────────────────────────────────────────────────

    /// A parser bound to a specific dialect via [`DialectDef`].
    pub struct TypedParser<D: DialectDef> {
        inner: RawParser<'static>,
        _marker: PhantomData<D>,
    }

    impl<D: DialectDef> TypedParser<D> {
        /// Create a parser with default configuration.
        pub fn new() -> Self {
            Self {
                inner: RawParser::builder(D::dialect()).build(),
                _marker: PhantomData,
            }
        }

        /// Create a builder for configuring the parser before construction.
        pub fn builder() -> crate::parser::session::BaseParserBuilder<'static> {
            RawParser::builder(D::dialect())
        }

        /// Bind source text and return a [`TypedStatementCursor`].
        pub fn parse<'a>(&'a mut self, source: &'a str) -> TypedStatementCursor<'a, D> {
            TypedStatementCursor {
                inner: self.inner.parse(source),
                _marker: PhantomData,
            }
        }
    }

    impl<D: DialectDef> Default for TypedParser<D> {
        fn default() -> Self {
            Self::new()
        }
    }

    // SAFETY: RawParser is Send, and TypedParser is a thin wrapper.
    unsafe impl<D: DialectDef> Send for TypedParser<D> {}

    // ── TypedStatementCursor ─────────────────────────────────────────────

    /// A streaming cursor over parsed SQL statements, yielding typed nodes.
    pub struct TypedStatementCursor<'a, D: DialectDef> {
        inner: RawStatementCursor<'a>,
        _marker: PhantomData<D>,
    }

    impl<'a, D: DialectDef> TypedStatementCursor<'a, D> {
        /// Parse the next SQL statement and return a typed AST node.
        pub fn next_statement(&mut self) -> Option<Result<D::Stmt<'a>, ParseError>> {
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
    }

    impl<'a, D: DialectDef> Iterator for TypedStatementCursor<'a, D> {
        type Item = Result<D::Stmt<'a>, ParseError>;

        fn next(&mut self) -> Option<Self::Item> {
            self.next_statement()
        }
    }

    // ── TypedTokenizer ───────────────────────────────────────────────────

    /// A tokenizer bound to a specific dialect via [`DialectDef`].
    pub struct TypedTokenizer<D: DialectDef> {
        inner: RawTokenizer,
        _marker: PhantomData<D>,
    }

    impl<D: DialectDef> TypedTokenizer<D> {
        /// Create a tokenizer with default configuration.
        pub fn new() -> Self {
            Self {
                inner: RawTokenizer::builder(*D::dialect()).build(),
                _marker: PhantomData,
            }
        }

        /// Bind source text and return a [`TypedTokenCursor`].
        pub fn tokenize<'a>(&'a mut self, source: &'a str) -> TypedTokenCursor<'a, D> {
            TypedTokenCursor {
                inner: self.inner.tokenize(source),
                _marker: PhantomData,
            }
        }
    }

    impl<D: DialectDef> Default for TypedTokenizer<D> {
        fn default() -> Self {
            Self::new()
        }
    }

    // SAFETY: RawTokenizer is Send, and TypedTokenizer is a thin wrapper.
    unsafe impl<D: DialectDef> Send for TypedTokenizer<D> {}

    // ── TypedTokenCursor ─────────────────────────────────────────────────

    /// An active tokenizer cursor yielding typed tokens.
    pub struct TypedTokenCursor<'a, D: DialectDef> {
        inner: RawTokenCursor<'a>,
        _marker: PhantomData<D>,
    }

    impl<'a, D: DialectDef> Iterator for TypedTokenCursor<'a, D> {
        type Item = (D::TokenType, &'a str);

        fn next(&mut self) -> Option<Self::Item> {
            let raw = self.inner.next()?;
            Some((D::token_from_raw(raw.token_type), raw.text))
        }
    }

    // ── TypedFormatter ───────────────────────────────────────────────────

    /// A formatter bound to a specific dialect via [`DialectDef`].
    #[cfg(feature = "fmt")]
    pub struct TypedFormatter<D: DialectDef> {
        inner: crate::fmt::formatter::Formatter<'static>,
        _marker: PhantomData<D>,
    }

    #[cfg(feature = "fmt")]
    impl<D: DialectDef> Default for TypedFormatter<D> {
        fn default() -> Self {
            Self::new()
        }
    }

    #[cfg(feature = "fmt")]
    impl<D: DialectDef> TypedFormatter<D> {
        /// Create a formatter with default configuration.
        pub fn new() -> Self {
            Self {
                inner: crate::fmt::formatter::Formatter::builder(D::dialect()).build(),
                _marker: PhantomData,
            }
        }

        /// Format SQL source text.
        pub fn format(&mut self, source: &str) -> Result<String, ParseError> {
            self.inner.format(source)
        }
    }
}

// ── Shared field extraction ────────────────────────────────────────────

use dialect::ffi::{FIELD_BOOL, FIELD_ENUM, FIELD_FLAGS, FIELD_NODE_ID, FIELD_SPAN, FieldMeta};
use dialect::Dialect;
use parser::nodes::{FieldVal, NodeId, SourceSpan};

/// Fill a `Fields` buffer by extracting all fields from a raw node pointer.
///
/// # Safety
/// `ptr` must point to a valid node struct matching `tag`'s metadata in `dialect`.
pub(crate) unsafe fn extract_fields<'a>(
    dialect: &Dialect<'_>,
    ptr: *const u8,
    tag: u32,
    source: &'a str,
) -> parser::nodes::Fields<'a> {
    let meta = dialect.field_meta(tag);
    let mut fields = parser::nodes::Fields::new();
    for m in meta {
        fields.push(unsafe { extract_field_val(ptr, m, source) });
    }
    fields
}

/// Extract a single field value from a raw node pointer using field metadata.
///
/// # Safety
/// `ptr` must point to a valid node struct whose field at `m.offset` has
/// the type indicated by `m.kind`.
pub(crate) unsafe fn extract_field_val<'a>(
    ptr: *const u8,
    m: &FieldMeta,
    source: &'a str,
) -> FieldVal<'a> {
    // SAFETY: All operations below are covered by the function-level safety
    // contract: `ptr` is a valid arena node and `m` describes its field layout.
    unsafe {
        let field_ptr = ptr.add(m.offset as usize);
        match m.kind {
            FIELD_NODE_ID => FieldVal::NodeId(NodeId(*(field_ptr as *const u32))),
            FIELD_SPAN => {
                let span = &*(field_ptr as *const SourceSpan);
                if span.length == 0 {
                    FieldVal::Span("", 0)
                } else {
                    FieldVal::Span(span.as_str(source), span.offset)
                }
            }
            FIELD_BOOL => FieldVal::Bool(*(field_ptr as *const u32) != 0),
            FIELD_FLAGS => FieldVal::Flags(*field_ptr),
            FIELD_ENUM => FieldVal::Enum(*(field_ptr as *const u32)),
            _ => panic!("unknown C field kind: {}", m.kind),
        }
    }
}

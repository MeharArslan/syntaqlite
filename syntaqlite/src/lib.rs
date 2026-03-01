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
//! - [`ast`] — SQLite-specific typed AST nodes and the top-level
//!   [`Stmt`](ast::Stmt) enum.
//! - [`dialect`] — The opaque [`Dialect`] handle, the
//!   [`sqlite()`](dialect::sqlite) dialect accessor, and
//!   semantic [`TokenCategory`](dialect::TokenCategory) enum.
//! - [`raw`] — Dialect-agnostic building blocks for external dialect crates
//!   (raw parsers, tokenizers, node types, and the [`Dialect`]
//!   handle).
//! - [`fmt`] — Formatter configuration ([`FormatConfig`](fmt::FormatConfig),
//!   [`KeywordCase`](fmt::KeywordCase)).
//! - [`validation`] — Validator configuration, diagnostic types, and
//!   schema context.
//! - [`embedded`] — Extract and validate SQL from Python f-strings and
//!   TypeScript template literals.
//! - [`lsp`] — [`AnalysisHost`](lsp::AnalysisHost) for editor integrations.

pub(crate) use syntaqlite_parser::ast_traits;
pub mod parser;

// ── Top-level API ─────────────────────────────────────────────────────
//
// Only the 5 primary user-facing types are re-exported at the crate root.
// Everything else lives in its host module (parser::*, fmt::*, validation::*).

#[cfg(feature = "sqlite")]
pub use sqlite::wrappers::{
    Parser, ParserBuilder, StatementCursor, Token, TokenCursor, Tokenizer, TokenizerBuilder,
};

pub use parser::token_parser::RawIncrementalParser as IncrementalParser;

// ── Formatter ────────────────────────────────────────────────────────────

#[cfg(feature = "fmt")]
pub mod fmt;
#[doc(inline)]
#[cfg(feature = "fmt")]
pub use fmt::formatter::Formatter;

// ── Dialect ──────────────────────────────────────────────────────────────

pub(crate) mod catalog;

pub mod dialect;
pub use dialect::Dialect;

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

pub(crate) mod sqlite;

/// Typed AST nodes for the SQLite dialect.
///
/// Re-exports from the internal SQLite dialect module. Each SQL statement
/// type (e.g. `SELECT`, `INSERT`) has a corresponding struct with typed
/// accessors. The top-level enum is [`Stmt`](ast::Stmt).
#[cfg(feature = "sqlite")]
pub mod ast {
    pub use syntaqlite_parser_sqlite::ast::*;
}

#[cfg(feature = "sqlite")]
pub use syntaqlite_parser_sqlite::tokens::TokenType;

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
    pub use crate::parser::session::ErrorSpan;
    pub use crate::parser::session::RawNodeReader;
    pub use crate::parser::session::RawParser;
    pub use crate::parser::session::RawStatementCursor;

    pub use crate::parser::tokenizer::RawToken;
    pub use crate::parser::tokenizer::RawTokenCursor;
    pub use crate::parser::tokenizer::RawTokenizer;

    pub use crate::parser::token_parser::RawIncrementalCursor;
    pub use crate::parser::token_parser::RawIncrementalParser;

    // ── Node / field types ───────────────────────────────────────────────
    pub use crate::parser::nodes::{ArenaNode, FieldVal, Fields, NodeId, NodeList, SourceSpan};
    pub use crate::parser::session::{NodeRef, ParseError};
    pub use crate::parser::typed_list::{FromArena, TypedList};

    // ── Token metadata ───────────────────────────────────────────────────
    pub use crate::parser::ffi::{
        Comment, CommentKind, TOKEN_FLAG_AS_FUNCTION, TOKEN_FLAG_AS_ID, TOKEN_FLAG_AS_TYPE,
    };

    // ── Typed wrappers (for external dialect crates) ─────────────────────
    pub use crate::parser::typed::{
        DialectTokenType, TypedParser, TypedParserBuilder, TypedStatementCursor, TypedToken,
        TypedTokenCursor, TypedTokenizer, TypedTokenizerBuilder,
    };

    // ── AST trait definitions ────────────────────────────────────────────
    pub use crate::ast_traits::*;

    // ── Builders ─────────────────────────────────────────────────────────

    /// Builder types for dialect-agnostic APIs.
    pub mod builders {
        pub use crate::parser::session::RawParserBuilder;
        pub use crate::parser::token_parser::RawIncrementalParserBuilder;
        pub use crate::parser::tokenizer::RawTokenizerBuilder;

        #[cfg(feature = "fmt")]
        pub use crate::fmt::formatter::FormatterBuilder;

        #[cfg(feature = "validation")]
        pub use crate::validation::ValidatorBuilder;
    }

    // ── Dialect handle ────────────────────────────────────────────────────

    pub use crate::dialect::Dialect;
    pub use crate::dialect::ffi::Dialect as FfiDialect;
}

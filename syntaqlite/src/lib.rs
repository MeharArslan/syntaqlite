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
//! - [`ext`] — Dialect-agnostic building blocks for external dialect crates
//!   (raw parsers, tokenizers, node types, and the [`Dialect`]
//!   handle).
//! - [`fmt`] — Formatter configuration ([`FormatConfig`](fmt::FormatConfig),
//!   [`KeywordCase`](fmt::KeywordCase)).
//! - [`validation`] — Validator configuration, diagnostic types, and
//!   schema context.
//! - [`embedded`] — Extract and validate SQL from Python f-strings and
//!   TypeScript template literals.
//! - [`lsp`] — [`AnalysisHost`](lsp::AnalysisHost) for editor integrations.

pub mod parser;

// ── Top-level API ─────────────────────────────────────────────────────
//
// Only the 5 primary user-facing types are re-exported at the crate root.
// Everything else lives in its host module (parser::*, fmt::*, validation::*).

#[cfg(feature = "sqlite")]
pub type Parser = crate::parser::typed::Parser<'static, syntaqlite_parser_sqlite::SqliteNodeFamily>;
#[cfg(feature = "sqlite")]
pub type ParserBuilder =
    crate::parser::typed::ParserBuilder<'static, syntaqlite_parser_sqlite::SqliteNodeFamily>;
#[cfg(feature = "sqlite")]
pub type StatementCursor<'a> =
    crate::parser::typed::StatementCursor<'a, syntaqlite_parser_sqlite::SqliteNodeFamily>;
#[cfg(feature = "sqlite")]
pub type Tokenizer =
    crate::parser::typed::Tokenizer<'static, syntaqlite_parser_sqlite::SqliteNodeFamily>;
#[cfg(feature = "sqlite")]
pub type TokenizerBuilder =
    crate::parser::typed::TokenizerBuilder<'static, syntaqlite_parser_sqlite::SqliteNodeFamily>;
#[cfg(feature = "sqlite")]
pub type Token<'a> =
    crate::parser::typed::Token<'a, syntaqlite_parser_sqlite::SqliteNodeFamily>;
#[cfg(feature = "sqlite")]
pub type TokenCursor<'a> =
    crate::parser::typed::TokenCursor<'a, syntaqlite_parser_sqlite::SqliteNodeFamily>;
#[cfg(feature = "sqlite")]
pub type IncrementalParser = crate::parser::typed::IncrementalParser<
    'static,
    syntaqlite_parser_sqlite::SqliteNodeFamily,
>;
#[cfg(feature = "sqlite")]
pub type IncrementalParserBuilder = crate::parser::typed::IncrementalParserBuilder<
    'static,
    syntaqlite_parser_sqlite::SqliteNodeFamily,
>;
#[cfg(feature = "sqlite")]
pub type IncrementalCursor<'a> =
    crate::parser::typed::IncrementalCursor<'a, syntaqlite_parser_sqlite::SqliteNodeFamily>;

// ── Formatter ────────────────────────────────────────────────────────────

#[cfg(feature = "fmt")]
pub mod fmt;
#[doc(inline)]
#[cfg(feature = "fmt")]
pub use fmt::formatter::Formatter;

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

pub mod dialect;

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

/// Dialect-agnostic ("ext") API for building custom dialect integrations.
///
/// Most users should prefer the top-level SQLite types ([`Parser`],
/// [`Formatter`], etc.). This module exposes the lower-level building
/// blocks that external dialect crates need.
pub mod ext {
    // ── Parser types ─────────────────────────────────────────────────────
    pub use syntaqlite_parser::RawParser;
    pub use syntaqlite_parser::RawParserBuilder;
    pub use syntaqlite_parser::RawStatementCursor;

    pub use syntaqlite_parser::RawToken;
    pub use syntaqlite_parser::RawTokenCursor;
    pub use syntaqlite_parser::RawTokenizer;

    pub use syntaqlite_parser::RawIncrementalCursor;
    pub use syntaqlite_parser::RawIncrementalParser;
    pub use syntaqlite_parser::RawIncrementalParserBuilder;

    // ── Node / field types ───────────────────────────────────────────────
    pub use syntaqlite_parser::ErrorSpan;
    pub use syntaqlite_parser::NodeRef;
    pub use syntaqlite_parser::ParseError;
    pub use syntaqlite_parser::TypedList;
    pub use syntaqlite_parser::{ArenaNode, FieldVal, Fields, NodeId, NodeList, SourceSpan};
    pub use syntaqlite_parser::{DialectNodeType, DialectTokenType};

    #[cfg(feature = "json")]
    pub use crate::parser::node_ref_json::NodeRefJsonExt;

    // ── Token metadata ───────────────────────────────────────────────────
    pub use syntaqlite_parser::{
        Comment, CommentKind, TOKEN_FLAG_AS_FUNCTION, TOKEN_FLAG_AS_ID, TOKEN_FLAG_AS_TYPE,
    };

    // ── AST trait definitions ────────────────────────────────────────────
    pub use syntaqlite_parser::ast_traits::*;

    /// Builder types for dialect-agnostic APIs.
    pub mod builders {
        pub use syntaqlite_parser::RawTokenizerBuilder;

        #[cfg(feature = "fmt")]
        pub use crate::fmt::formatter::FormatterBuilder;

        #[cfg(feature = "validation")]
        pub use crate::validation::ValidatorBuilder;
    }

    // ── Dialect handle ────────────────────────────────────────────────────

    pub use syntaqlite_parser::Dialect;
    pub use syntaqlite_parser::FfiDialect;
    pub use syntaqlite_parser::NodeFamily;
    pub use syntaqlite_parser::RawDialect;
}

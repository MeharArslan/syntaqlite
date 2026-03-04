// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::similar_names))]

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
//! let parser = Parser::new();
//! let mut cursor = parser.parse("SELECT 1 + 2; CREATE TABLE t(x)");
//! while let Some(result) = cursor.next_statement() {
//!     let stmt = result.expect("parse error");
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
//! **Validate** SQL:
//!
//! ```
//! use syntaqlite::{SemanticAnalyzer, DatabaseCatalog};
//!
//! let catalog = DatabaseCatalog::default();
//! let mut analyzer = SemanticAnalyzer::new();
//! let diags = analyzer.diagnostics("SELEC 1", &catalog);
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
//! and [`SemanticAnalyzer`] — are re-exported at the crate root. With the
//! `sqlite` feature (enabled by default), each provides a `::new()` constructor
//! for the built-in SQLite dialect.
//!
//! For lower-level or dialect-agnostic access, see:
//!
//! - [`ast`] — SQLite-specific typed AST nodes and the top-level
//!   [`Stmt`](ast::Stmt) enum.
//! - [`dialect`] — The opaque `TypedDialectEnv` handle, the
//!   [`sqlite()`](dialect::sqlite) dialect accessor, and
//!   semantic [`TokenCategory`](dialect::TokenCategory) enum.
//! - [`fmt`] — Formatter configuration ([`FormatConfig`],
//!   [`KeywordCase`]).
//! - [`semantic`] — Semantic analysis: diagnostics, catalog, validation config.
//! - [`embedded`] — Extract and validate SQL from Python f-strings and
//!   TypeScript template literals.
//! - [`lsp`] — [`LspHost`](lsp::LspHost) for editor integrations.

pub(crate) mod parser;

// ── Concrete SQLite API ─────────────────────────────────────────────────
//
// The concrete types — `Parser`, `Tokenizer`, `StatementCursor`, etc. —
// bake in the SQLite dialect so call sites never need type parameters.
// TypedDialectEnv-generic versions live in the `dialect` module.

#[cfg(feature = "sqlite")]
mod sqlite_api;
#[cfg(feature = "sqlite")]
pub use sqlite_api::{Parser, StatementCursor, Token, TokenCursor, Tokenizer};

pub mod incremental;

// ── Formatter ────────────────────────────────────────────────────────────

#[cfg(feature = "fmt")]
pub mod fmt;
#[doc(inline)]
#[cfg(feature = "fmt")]
pub use fmt::formatter::Formatter;

// ── Semantic analysis ────────────────────────────────────────────────────

#[cfg(feature = "validation")]
pub mod semantic;

// ── Semantic re-exports at crate root ────────────────────────────────
#[cfg(feature = "validation")]
pub use semantic::DatabaseCatalog;
#[cfg(feature = "validation")]
pub use semantic::SemanticAnalyzer;
#[cfg(feature = "validation")]
pub use semantic::SemanticModel;
#[cfg(feature = "validation")]
pub use semantic::ValidationConfig;

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

#[cfg(feature = "fmt")]
pub use fmt::{FormatConfig, KeywordCase};

#[cfg(feature = "json")]
pub use crate::parser::node_ref_json::NodeRefJsonExt;

// ── Grammar-agnostic parser types (Raw* aliases) ─────────────────────────
//
// `syntaqlite_parser` now uses clean names (`Parser`, `Tokenizer`, …).
// These `Raw*` re-exports preserve the old qualified path for callers that
// previously wrote `syntaqlite::RawParser` or `syntaqlite_parser::RawParser`.

pub use syntaqlite_parser::IncrementalCursor as RawIncrementalCursor;
pub use syntaqlite_parser::IncrementalParser as RawIncrementalParser;
pub use syntaqlite_parser::NodeId as RawNodeId;
pub use syntaqlite_parser::ParseResult as RawParseResult;
pub use syntaqlite_parser::Parser as RawParser;
pub use syntaqlite_parser::StatementCursor as RawStatementCursor;
pub use syntaqlite_parser::Token as RawToken;
pub use syntaqlite_parser::TokenCursor as RawTokenCursor;
pub use syntaqlite_parser::Tokenizer as RawTokenizer;

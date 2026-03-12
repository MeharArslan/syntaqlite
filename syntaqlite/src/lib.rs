// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![cfg_attr(test, expect(clippy::unwrap_used))]

//! Fast, accurate SQL tooling for `SQLite` and its dialects.
//!
//! This crate provides parsing, formatting, and semantic validation for SQL,
//! built on `SQLite`'s own tokenizer and grammar rules. Four design principles
//! guide the library:
//!
//! - **Reliability** — uses `SQLite`'s own grammar rules; formatting is round-trip safe and validation mirrors real engine behaviour.
//! - **Speed** — all core types ([`Formatter`], [`SemanticAnalyzer`], [`Catalog`]) are designed for reuse across many inputs without re-allocation.
//! - **Portability** — the core formatting and validation engine has no runtime dependencies beyond the standard library; optional features (`lsp`, `serde`) pull in additional crates.
//! - **Flexibility** — supports multiple database dialects that extend `SQLite`'s grammar with their own tokens and rules.
//!
//! # Parsing
//!
//! Use [`Parser`] to parse SQL source text into a typed AST:
//!
//! ```rust
//! use syntaqlite::{Parser, ParseOutcome, ParseErrorKind};
//!
//! let parser = Parser::new();
//! let mut session = parser.parse("SELECT 1; SELECT 2");
//! loop {
//!     match session.next() {
//!         ParseOutcome::Ok(stmt) => println!("{:?}", stmt.root()),
//!         ParseOutcome::Err(e) => {
//!             eprintln!("parse error: {}", e.message());
//!             if e.kind() == ParseErrorKind::Fatal { break; }
//!         }
//!         ParseOutcome::Done => break,
//!     }
//! }
//! ```
//!
//! # Validation
//!
//! Use [`SemanticAnalyzer`] to check SQL against a known schema. The analyzer
//! produces a [`SemanticModel`] containing structured [`Diagnostic`] values
//! with byte-offset spans and "did you mean?" suggestions.
//!
//! ```rust
//! use syntaqlite::{
//!     SemanticAnalyzer, Catalog, CatalogLayer, ValidationConfig, sqlite_dialect,
//! };
//!
//! let mut analyzer = SemanticAnalyzer::new();
//! let mut catalog = Catalog::new(sqlite_dialect());
//!
//! // Register a table so the analyzer can resolve column references.
//! catalog.layer_mut(CatalogLayer::Database)
//!     .insert_table("users", Some(vec!["id".into(), "name".into()]), false);
//!
//! let config = ValidationConfig::default();
//! let model = analyzer.analyze("SELECT id, name FROM users", &catalog, &config);
//!
//! // All names resolve — no diagnostics.
//! assert!(model.diagnostics().is_empty());
//! ```
//!
//! For richer output, use [`DiagnosticRenderer`](util::DiagnosticRenderer) to produce rustc-style
//! error messages with source context and underlines.
//!
//! # Formatting
//!
//! Use [`Formatter`] to pretty-print SQL with consistent style. The formatter
//! parses each statement, runs a bytecode interpreter over the AST, and
//! renders the result with a Wadler-style pretty-printer.
//!
//! ```rust
//! # use syntaqlite::{Formatter, FormatConfig, KeywordCase};
//! let mut fmt = Formatter::with_config(
//!     &FormatConfig::default()
//!         .with_keyword_case(KeywordCase::Lower)
//!         .with_line_width(60),
//! );
//!
//! let output = fmt.format("select id,name from users where active=1").unwrap();
//! assert!(output.starts_with("select"));
//! assert!(output.contains("from"));
//! ```
//!
//! See [`FormatConfig`] for all available options (line width, indent width,
//! keyword casing, semicolons).
//!
//! # Advanced
//!
//! ## Tokenizing
//!
//! Use [`Tokenizer`] to break SQL source text into [`Token`]s:
//!
//! ```rust
//! let tokenizer = syntaqlite::Tokenizer::new();
//! for token in tokenizer.tokenize("SELECT 1") {
//!     println!("{:?}: {:?}", token.token_type(), token.text());
//! }
//! ```
//!
//! ## Incremental Parsing
//!
//! Use [`IncrementalParseSession`] when SQL arrives token-by-token
//! (for example in editors and completion engines):
//!
//! ```rust
//! use syntaqlite::{Parser, TokenType};
//!
//! let parser = Parser::new();
//! let mut session = parser.incremental_parse("SELECT 1");
//!
//! assert!(session.feed_token(TokenType::Select, 0..6).is_none());
//! assert!(session.feed_token(TokenType::Integer, 7..8).is_none());
//!
//! let stmt = session.finish().and_then(Result::ok).unwrap();
//! let _ = stmt.root();
//! ```
//!
//! # Features
//!
//! - `sqlite` *(default)*: enables the built-in `SQLite` grammar, [`Dialect`],
//!   and re-exports [`Parser`], [`Tokenizer`], and typed AST [`nodes`].
//! - `fmt` *(default)*: enables [`Formatter`], [`FormatConfig`], and
//!   [`KeywordCase`].
//! - `validation` *(default)*: enables [`SemanticAnalyzer`], [`Catalog`],
//!   [`Diagnostic`], and related types.
//! - `lsp`: enables [`LspServer`](lsp::LspServer) and [`lsp::LspHost`] for editor integration.
//! - `experimental-embedded`: enables [`embedded`] SQL extraction from Python
//!   and TypeScript/JavaScript source files.
//! - `serde`: adds `Serialize`/`Deserialize` impls for diagnostics and AST
//!   nodes.
//! - `serde-json`: adds JSON convenience helpers (catalog from JSON, AST dump).
//!
//! # Choosing an API
//!
//! - Use [`Parser`] and [`Tokenizer`] for parsing and tokenizing SQL.
//! - Use [`SemanticAnalyzer`] + [`Catalog`] when you need to validate SQL
//!   against a database schema (table/column/function resolution).
//! - Use [`Formatter`] when you need to pretty-print or normalize SQL text.
//! - Use [`LspServer`](lsp::LspServer) (requires the `lsp` feature) to embed a full
//!   Language Server Protocol implementation in an editor or tool.
//! - Use [`typed`] when building reusable code over known generated grammars.
//! - Use [`any`] when grammar choice happens at runtime or crosses
//!   FFI/plugin boundaries.

// Temporarily disabled during refactor except for formatter dependency chain.
#[cfg(feature = "fmt")]
pub mod dialect;

#[cfg(feature = "fmt")]
pub(crate) mod fmt;

// Incrementally re-enabled during refactor.
#[cfg(feature = "validation")]
pub(crate) mod semantic;

// `sqlite` module is always present; individual sub-modules are gated inside it.
pub(crate) mod sqlite;

#[cfg(feature = "lsp")]
pub mod lsp;

/// Embedded SQL extraction from host language sources.
///
/// # Experimental
///
/// This module is experimental and its API may change in future releases.
/// Enable with the `experimental-embedded` cargo feature.
#[cfg(feature = "experimental-embedded")]
pub mod embedded;

// ── Public API ────────────────────────────────────────────────────────────────

#[cfg(feature = "fmt")]
pub use fmt::formatter::Formatter;
#[cfg(feature = "fmt")]
pub use fmt::{FormatConfig, FormatError, KeywordCase};
#[cfg(feature = "validation")]
pub use semantic::{
    AnalysisMode, AritySpec, Catalog, CatalogLayer, CatalogLayerContents, Diagnostic,
    DiagnosticMessage, FunctionCategory, Help, SemanticAnalyzer, SemanticModel, Severity,
    ValidationConfig,
};
#[cfg(feature = "sqlite")]
pub use sqlite::dialect::Dialect;
/// Returns the built-in `SQLite` dialect handle.
///
/// Returns a [`Dialect`] (the SQLite-specific newtype). Call `.erase()` or
/// `.into()` to obtain an [`AnyDialect`](any::AnyDialect) when a type-erased handle is needed.
#[cfg(feature = "sqlite")]
pub fn sqlite_dialect() -> Dialect {
    Dialect::new()
}

pub mod util;

// Shared parser utility types used across both `any` and `typed` modules.
pub use syntaqlite_syntax::any::MacroRegion;
pub use syntaqlite_syntax::{CommentKind, ParserConfig, ParserTokenFlags};

// SQLite parser, tokenizer, and token types re-exported for direct use.
#[cfg(feature = "sqlite")]
pub use syntaqlite_syntax::{
    IncrementalParseSession, ParseError, ParseErrorKind, ParseOutcome, ParseSession,
    ParsedStatement, Parser, ParserToken, Token, TokenType, Tokenizer,
};

/// Type-erased (grammar-agnostic) parser and tokenizer types.
pub mod any {
    pub use syntaqlite_syntax::any::*;

    #[cfg(feature = "fmt")]
    pub use crate::dialect::AnyDialect;
}

/// Typed (grammar-parameterized) parser and tokenizer infrastructure.
pub mod typed {
    pub use syntaqlite_syntax::typed::*;

    #[cfg(feature = "fmt")]
    pub use crate::dialect::TypedDialect;
}

/// Generated typed AST nodes for the built-in `SQLite` grammar.
#[cfg(feature = "sqlite")]
pub mod nodes {
    pub use syntaqlite_syntax::nodes::*;
}

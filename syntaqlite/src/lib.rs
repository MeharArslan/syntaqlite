// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![cfg_attr(test, expect(clippy::unwrap_used))]

//! Fast, accurate SQL tooling for `SQLite` and its dialects.

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
pub use dialect::{AnyDialect, TypedDialect};
#[cfg(feature = "fmt")]
pub use fmt::formatter::Formatter;
#[cfg(feature = "fmt")]
pub use fmt::{FormatConfig, FormatError, KeywordCase};
#[cfg(feature = "lsp")]
pub use lsp::LspServer;
#[cfg(feature = "validation")]
pub use semantic::{
    Catalog, Diagnostic, DiagnosticMessage, DiagnosticRenderer, Help, SemanticAnalyzer,
    SemanticModel, Severity, SourceContext, ValidationConfig,
};
#[cfg(feature = "sqlite")]
pub use sqlite::dialect::Dialect;
/// Returns the built-in `SQLite` dialect handle.
///
/// Returns a [`Dialect`] (the SQLite-specific newtype). Call `.erase()` or
/// `.into()` to obtain an [`AnyDialect`] when a type-erased handle is needed.
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
}

/// Typed (grammar-parameterized) parser and tokenizer infrastructure.
pub mod typed {
    pub use syntaqlite_syntax::typed::*;
}

/// Generated typed AST nodes for the built-in `SQLite` grammar.
#[cfg(feature = "sqlite")]
pub mod nodes {
    pub use syntaqlite_syntax::nodes::*;
}

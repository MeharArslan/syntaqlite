// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![cfg_attr(test, expect(clippy::unwrap_used, clippy::similar_names))]

//! Fast, accurate SQL tooling for `SQLite` and its dialects.

// Temporarily disabled during refactor except for formatter dependency chain.
#[cfg(feature = "fmt")]
pub mod dialect;

#[cfg(feature = "fmt")]
pub(crate) mod fmt;

// Incrementally re-enabled during refactor.
#[cfg(feature = "validation")]
pub(crate) mod semantic;

#[cfg(feature = "sqlite")]
pub(crate) mod sqlite;

#[cfg(feature = "lsp")]
pub mod lsp;

#[cfg(feature = "embedded")]
pub mod embedded;

// ── Public API ────────────────────────────────────────────────────────────────

#[cfg(feature = "fmt")]
pub use dialect::{AnyDialect, Dialect};
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
/// Returns the built-in `SQLite` dialect handle.
#[cfg(feature = "sqlite")]
pub fn sqlite_dialect() -> Dialect {
    sqlite::dialect::dialect()
}

pub mod util;

// Shared parser utility types used across both `any` and `typed` modules.
pub use syntaqlite_syntax::any::MacroRegion;
pub use syntaqlite_syntax::{CommentKind, ParserConfig, ParserTokenFlags};

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

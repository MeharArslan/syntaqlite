// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::similar_names))]

//! Fast, accurate SQL tooling for `SQLite` and its dialects.

// Temporarily disabled during refactor except for formatter dependency chain.
#[cfg(feature = "fmt")]
pub(crate) mod dialect;

#[cfg(feature = "fmt")]
pub(crate) mod fmt;

// Incrementally re-enabled during refactor.
#[cfg(feature = "validation")]
pub(crate) mod semantic;

#[cfg(feature = "sqlite")]
pub(crate) mod sqlite;

// ── Public API ────────────────────────────────────────────────────────────────

#[cfg(feature = "fmt")]
pub use fmt::formatter::Formatter;
#[cfg(feature = "fmt")]
pub use fmt::{FormatConfig, FormatError, KeywordCase};
#[cfg(feature = "validation")]
pub use semantic::{Diagnostic, DiagnosticMessage, Help, Severity, ValidationConfig};

// Shared parser utility types used across both `any` and `typed` modules.
pub use syntaqlite_syntax::any::MacroRegion;
pub use syntaqlite_syntax::{CommentKind, ParserConfig, ParserTokenFlags};

/// Type-erased (grammar-agnostic) parser and tokenizer types.
///
/// Use these when working across multiple dialects, or when the grammar is not
/// known at compile time. For `SQLite` or a specific known grammar, prefer the
/// types in [`typed`] instead.
pub mod any {
    pub use syntaqlite_syntax::any::{
        // Grammar inspection
        AnyGrammar,
        AnyIncrementalParseSession,
        // AST
        AnyNode,
        AnyNodeId,
        AnyParseError,
        AnyParseSession,
        AnyParsedStatement,
        // Parser
        AnyParser,
        AnyParserToken,
        // Tokenizer
        AnyToken,
        AnyTokenizer,
        FieldKind,
        FieldMeta,
        FieldValue,
        KeywordEntry,
        MacroRegion,
        NodeFields,
    };
}

/// Typed (grammar-parameterized) parser and tokenizer infrastructure.
///
/// Use these when the dialect grammar `G` is known at compile time. For the
/// built-in `SQLite` dialect this is already wired up; access it via
/// [`crate::sqlite`].
pub mod typed {
    pub use syntaqlite_syntax::typed::{
        // Grammar traits
        GrammarNodeType,
        GrammarTokenType,
        TypedGrammar,
        TypedIncrementalParseSession,
        TypedNodeId,
        TypedNodeList,
        TypedParseError,
        TypedParseSession,
        TypedParsedStatement,
        // Parser
        TypedParser,
        TypedParserToken,
        // Tokenizer
        TypedToken,
        TypedTokenizer,
    };
}

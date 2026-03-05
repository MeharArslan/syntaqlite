// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Opaque precomputed SQL representation.
//!
//! [`SemanticModel`] is produced by [`SemanticAnalyzer::prepare()`] and holds
//! the owned source text, accumulated token stream, comments, and parse errors.
//! It has no public methods — callers pass it to `_prepared` methods on the
//! analyzer.

use syntaqlite_syntax::{ParserTokenFlags, TokenType};

// ── Types shared between semantic and LSP layers ─────────────────────

/// Semantic completion context derived from parser stack state.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionContext {
    /// Could not determine context.
    Unknown = 0,
    /// Cursor is in an expression position (functions/values expected).
    Expression = 1,
    /// Cursor is in a table-reference position (table/view names expected).
    TableRef = 2,
}

impl CompletionContext {
    pub(crate) fn from_parser(v: syntaqlite_syntax::CompletionContext) -> Self {
        match v {
            syntaqlite_syntax::CompletionContext::Expression => Self::Expression,
            syntaqlite_syntax::CompletionContext::TableRef => Self::TableRef,
            syntaqlite_syntax::CompletionContext::Unknown => Self::Unknown,
        }
    }
}

/// Expected tokens and semantic context at a cursor position.
#[derive(Debug)]
pub struct CompletionInfo {
    /// Terminal token types valid at the cursor.
    pub tokens: Vec<TokenType>,
    /// Semantic context (expression vs table-ref).
    pub context: CompletionContext,
}

/// A semantic token for syntax highlighting.
#[derive(Debug, Clone)]
pub struct SemanticToken {
    /// Byte offset in the source text.
    pub offset: usize,
    /// Length in bytes.
    pub length: usize,
    /// Token category.
    pub category: crate::dialect::TokenCategory,
}

// ── Stored per-statement token data ──────────────────────────────────

/// A token position recorded during parsing. Stored in [`SemanticModel`].
#[derive(Debug, Clone)]
pub(crate) struct StoredToken {
    pub(crate) offset: usize,
    pub(crate) length: usize,
    pub(crate) token_type: TokenType,
    pub(crate) flags: ParserTokenFlags,
}

/// A comment position recorded during parsing. Stored in [`SemanticModel`].
#[derive(Debug, Clone)]
pub(crate) struct StoredComment {
    pub(crate) offset: usize,
    pub(crate) length: usize,
}

/// A parse error recorded during parsing. Stored in [`SemanticModel`].
#[derive(Debug, Clone)]
pub(crate) struct StoredParseError {
    pub(crate) message: String,
    pub(crate) offset: Option<usize>,
    pub(crate) length: Option<usize>,
}

// ── SemanticModel ────────────────────────────────────────────────────

/// Opaque precomputed representation of parsed SQL.
///
/// Owns the source text, token stream, comments, and parse errors.
/// Produced only by [`SemanticAnalyzer::prepare()`](super::SemanticAnalyzer::prepare).
pub struct SemanticModel {
    pub(crate) source: String,
    pub(crate) tokens: Vec<StoredToken>,
    pub(crate) comments: Vec<StoredComment>,
    pub(crate) parse_errors: Vec<StoredParseError>,
}

impl SemanticModel {
    pub(crate) fn new(
        source: String,
        tokens: Vec<StoredToken>,
        comments: Vec<StoredComment>,
        parse_errors: Vec<StoredParseError>,
    ) -> Self {
        SemanticModel {
            source,
            tokens,
            comments,
            parse_errors,
        }
    }

    /// The source text bound to this model.
    pub(crate) fn source(&self) -> &str {
        &self.source
    }
}

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Result types for a single semantic analysis pass.

use syntaqlite_syntax::{ParserTokenFlags, TokenType};

use super::diagnostics::Diagnostic;
use syntaqlite_syntax::any::TokenCategory;

// ── Stored per-statement positions ───────────────────────────────────────────

/// A token position recorded during parsing.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct StoredToken {
    pub(crate) offset: usize,
    pub(crate) length: usize,
    pub(crate) token_type: TokenType,
    pub(crate) flags: ParserTokenFlags,
}

/// A comment position recorded during parsing.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct StoredComment {
    pub(crate) offset: usize,
    pub(crate) length: usize,
}

// ── Output types ──────────────────────────────────────────────────────────────

/// A semantic token for syntax highlighting.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct SemanticToken {
    /// Byte offset in the source text.
    pub offset: usize,
    /// Length in bytes.
    pub length: usize,
    /// Token category for highlighting.
    pub category: TokenCategory,
}

/// Semantic completion context derived from parser stack state.
#[allow(dead_code)]
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompletionContext {
    /// Could not determine context.
    Unknown = 0,
    /// Cursor is in an expression position (functions/values expected).
    Expression = 1,
    /// Cursor is in a table-reference position (table/view names expected).
    TableRef = 2,
}

#[allow(dead_code)]
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
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct CompletionInfo {
    /// Terminal token types valid at the cursor.
    pub tokens: Vec<TokenType>,
    /// Semantic context (expression vs table-ref).
    pub context: CompletionContext,
}

// ── SemanticModel ─────────────────────────────────────────────────────────────

/// Result of a single analysis pass.
///
/// Owns the source text, stored token/comment positions, and all diagnostics
/// (both parse errors and semantic issues). Produced by
/// [`SemanticAnalyzer::analyze`](super::analyzer::SemanticAnalyzer::analyze).
#[allow(dead_code)]
pub(crate) struct SemanticModel {
    pub(crate) source: String,
    pub(crate) tokens: Vec<StoredToken>,
    pub(crate) comments: Vec<StoredComment>,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

#[allow(dead_code)]
impl SemanticModel {
    pub(crate) fn source(&self) -> &str {
        &self.source
    }

    pub(crate) fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

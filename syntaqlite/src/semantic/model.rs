// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Opaque precomputed SQL representation.
//!
//! [`SemanticModel`] is produced by [`SemanticAnalyzer::prepare()`] and holds
//! the parser arena, source text, statement results, and cached token stream.
//! It has no public methods — callers pass it to `_prepared` methods on the
//! analyzer.

use syntaqlite_parser::{ParseError, RawNodeId, RawParseResult, RawParser, RawStatementCursor};

// ── Re-export `RawStatementCursor` lifetime change ───────────────────
// `RawStatementCursor<'d>` no longer borrows source text — it copies
// source internally.  `SemanticModel<'d>` therefore has a single
// lifetime (the dialect), making it trivially cacheable.

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
    pub(crate) fn from_raw(v: u32) -> Self {
        match v {
            1 => Self::Expression,
            2 => Self::TableRef,
            _ => Self::Unknown,
        }
    }
}

/// Expected tokens and semantic context at a cursor position.
#[derive(Debug)]
pub struct CompletionInfo {
    /// Terminal token IDs valid at the cursor.
    pub tokens: Vec<u32>,
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

// ── SemanticModel ────────────────────────────────────────────────────

/// Opaque precomputed representation of parsed SQL.
///
/// Owns the parser and its cursor so node IDs remain valid. Produced only by
/// [`SemanticAnalyzer::prepare()`](super::SemanticAnalyzer::prepare).
///
/// # Lifetimes
///
/// - `'a` — the source text passed to `prepare()`.
/// - `'d` — the dialect (for the common SQLite case this is `'static`).
pub struct SemanticModel<'d> {
    /// Keeps the C parser alive (via the Rc inside RawParser).
    _parser: RawParser<'d>,
    /// Exhausted cursor — kept alive for its reader (arena access).
    cursor: RawStatementCursor<'d>,
    pub(crate) stmts: Vec<Result<RawNodeId, ParseError>>,
}

impl<'d> SemanticModel<'d> {
    /// Construct a new model from a parser, its cursor, and collected results.
    pub(crate) fn new(
        parser: RawParser<'d>,
        cursor: RawStatementCursor<'d>,
        stmts: Vec<Result<RawNodeId, ParseError>>,
    ) -> Self {
        SemanticModel {
            _parser: parser,
            cursor,
            stmts,
        }
    }

    /// Get a [`RawParseResult`] for the parser's arena state.
    pub(crate) fn reader(&self) -> RawParseResult<'_> {
        self.cursor.reader()
    }

    /// The source text bound to this model.
    pub(crate) fn source(&self) -> &str {
        self.cursor.source()
    }
}

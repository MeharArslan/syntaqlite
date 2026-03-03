// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Opaque precomputed SQL representation.
//!
//! [`SemanticModel`] is produced by [`SemanticAnalyzer::prepare()`] and holds
//! the parser arena, source text, statement results, and cached token stream.
//! It has no public methods — callers pass it to `_prepared` methods on the
//! analyzer.

use syntaqlite_parser::{ParseError, RawNodeId, RawParseResult, RawParser, RawStatementCursor};

/// Opaque precomputed representation of parsed SQL.
///
/// Owns the parser and its cursor so node IDs remain valid. Produced only by
/// [`SemanticAnalyzer::prepare()`](super::SemanticAnalyzer::prepare).
///
/// # Lifetimes
///
/// - `'a` — the source text passed to `prepare()`.
/// - `'d` — the dialect (for the common SQLite case this is `'static`).
pub struct SemanticModel<'a, 'd: 'a> {
    /// Keeps the C parser alive (via the Rc inside RawParser).
    _parser: RawParser<'d>,
    /// Exhausted cursor — kept alive for its reader (arena access).
    cursor: RawStatementCursor<'a>,
    pub(crate) stmts: Vec<Result<RawNodeId, ParseError>>,
}

impl<'a, 'd: 'a> SemanticModel<'a, 'd> {
    /// Construct a new model from a parser, its cursor, and collected results.
    pub(crate) fn new(
        parser: RawParser<'d>,
        cursor: RawStatementCursor<'a>,
        stmts: Vec<Result<RawNodeId, ParseError>>,
    ) -> Self {
        SemanticModel {
            _parser: parser,
            cursor,
            stmts,
        }
    }

    /// Get a [`RawParseResult`] for the parser's arena state.
    pub(crate) fn reader(&self) -> RawParseResult<'a> {
        self.cursor.reader()
    }

    /// The source text bound to this model.
    pub(crate) fn source(&self) -> &'a str {
        self.cursor.source()
    }
}

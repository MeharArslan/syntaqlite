// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Opaque precomputed SQL representation.
//!
//! [`SemanticModel`] is produced by [`SemanticAnalyzer::prepare()`] and holds
//! the parser arena, source text, statement results, and cached token stream.
//! It has no public methods — callers pass it to `_prepared` methods on the
//! analyzer.

use syntaqlite_parser::{NodeId, ParseError, RawParser};

/// Opaque precomputed representation of parsed SQL.
///
/// Owns the parser arena so node IDs remain valid. Produced only by
/// [`SemanticAnalyzer::prepare()`](super::SemanticAnalyzer::prepare).
///
/// # Lifetime
///
/// The `'d` parameter tracks the dialect. For the common SQLite case this
/// is `'static`. A future version may erase this lifetime since the arena
/// is self-contained after parsing.
pub struct SemanticModel<'d> {
    pub(crate) source: String,
    pub(crate) parser: RawParser<'d>,
    pub(crate) stmts: Vec<Result<NodeId, ParseError>>,
}

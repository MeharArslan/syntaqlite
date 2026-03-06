// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Language-server support: analysis host, document management, and
//! protocol server.
//!
//! # Overview
//!
//! - [`LspHost`] — stateful document store with lazy per-document
//!   analysis (diagnostics, semantic tokens, completions, formatting).
//!   Delegates semantic validation to [`SemanticAnalyzer`](crate::semantic::SemanticAnalyzer).
//! - [`LspServer`] — stdio JSON-RPC server that drives an `LspHost`
//!   in response to LSP messages from an editor.

use syntaqlite_syntax::any::TokenCategory;

/// Semantic token type names in legend-index order, for use in LSP
/// `SemanticTokensLegend` and Monaco provider registration.
pub(crate) const SEMANTIC_TOKEN_LEGEND: &[&str] = &[
    "keyword",     // 0 — TokenCategory::Keyword
    "variable",    // 1 — TokenCategory::Variable
    "string",      // 2 — TokenCategory::String
    "number",      // 3 — TokenCategory::Number
    "operator",    // 4 — TokenCategory::Operator
    "comment",     // 5 — TokenCategory::Comment
    "punctuation", // 6 — TokenCategory::Punctuation
    "identifier",  // 7 — TokenCategory::Identifier
    "function",    // 8 — TokenCategory::Function
    "type",        // 9 — TokenCategory::Type
];

/// LSP-specific extension on [`TokenCategory`].
pub(crate) trait TokenCategoryExt {
    /// 0-based index into [`SEMANTIC_TOKEN_LEGEND`].
    /// Returns `None` for [`TokenCategory::Other`], which is never emitted.
    fn legend_index(self) -> Option<usize>;

    /// The LSP token type name for this category, or `None` for `Other`.
    fn legend_name(self) -> Option<&'static str>;
}

impl TokenCategoryExt for TokenCategory {
    fn legend_index(self) -> Option<usize> {
        match self {
            TokenCategory::Keyword => Some(0),
            TokenCategory::Variable => Some(1),
            TokenCategory::String => Some(2),
            TokenCategory::Number => Some(3),
            TokenCategory::Operator => Some(4),
            TokenCategory::Comment => Some(5),
            TokenCategory::Punctuation => Some(6),
            TokenCategory::Identifier => Some(7),
            TokenCategory::Function => Some(8),
            TokenCategory::Type => Some(9),
            TokenCategory::Other => None,
        }
    }

    fn legend_name(self) -> Option<&'static str> {
        self.legend_index().map(|i| SEMANTIC_TOKEN_LEGEND[i])
    }
}

// Public API starts here.
pub(crate) use host::LspHost;
pub(crate) use server::LspServer;

// Re-export shared types from semantic layer.
pub(crate) use crate::semantic::model::{CompletionContext, CompletionInfo};

// ── LSP-specific types ──────────────────────────────────────────────────

/// A completion item returned by [`LspHost::completion_items`].
#[derive(Debug, Clone)]
pub(crate) struct CompletionEntry {
    /// The label to display and insert.
    pub(crate) label: String,
    /// What kind of thing is being completed.
    pub(crate) kind: CompletionKind,
}

/// The kind of a [`CompletionEntry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompletionKind {
    Keyword,
    Function,
}

impl CompletionKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::Function => "function",
        }
    }
}
// Public API ends here.

mod host;
mod server;

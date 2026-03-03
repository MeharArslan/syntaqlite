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
//! - [`DocumentAnalysis`] — the result of analysing a single document;
//!   produced by [`DocumentAnalysis::compute`] and cached inside the host.
//! - [`LspServer`] — stdio JSON-RPC server that drives an `LspHost`
//!   in response to LSP messages from an editor.

// Public API starts here.
pub use analysis::DocumentAnalysis;
pub use host::LspHost;
pub use server::LspServer;

// ── Shared LSP types ──────────────────────────────────────────────────────

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

/// A completion item returned by [`LspHost::completion_items`].
#[derive(Debug, Clone)]
pub struct CompletionEntry {
    /// The label to display and insert.
    pub label: String,
    /// What kind of thing is being completed.
    pub kind: CompletionKind,
}

/// The kind of a [`CompletionEntry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Keyword,
    Function,
}

impl CompletionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::Function => "function",
        }
    }
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
// Public API ends here.

mod analysis;
mod host;
mod server;

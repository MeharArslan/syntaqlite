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

// Public API starts here.
pub use host::LspHost;
pub use server::LspServer;

// Re-export shared types from semantic layer.
pub use crate::semantic::{CompletionContext, CompletionInfo, SemanticToken};

// ── LSP-specific types ──────────────────────────────────────────────────

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
// Public API ends here.

mod host;
mod server;

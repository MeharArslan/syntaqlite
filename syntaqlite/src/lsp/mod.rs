// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Language-server analysis host.
//!
//! [`AnalysisHost`] manages a set of open documents and provides
//! diagnostics, semantic tokens, completions, and formatting in a
//! single interface suitable for driving an LSP server or in-editor
//! extension.

pub mod host;

pub use host::{AnalysisHost, CompletionContext, CompletionInfo, FormatError};

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

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

pub mod host;

pub use crate::dialect::TokenCategory;
pub use crate::validation::types::{AmbientContext, ColumnDef, FunctionDef, TableDef, ViewDef};
pub use crate::validation::types::{Diagnostic, Severity};
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

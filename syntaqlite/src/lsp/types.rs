// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

pub use crate::validation::types::{Diagnostic, Severity};

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

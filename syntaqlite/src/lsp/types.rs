// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

/// A diagnostic message associated with a source range.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Byte offset of the start of the diagnostic range.
    pub start_offset: usize,
    /// Byte offset of the end of the diagnostic range.
    pub end_offset: usize,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Severity level.
    pub severity: Severity,
}

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
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

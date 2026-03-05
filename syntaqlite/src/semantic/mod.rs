// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Semantic analysis (incrementally re-enabled).

pub(crate) mod diagnostics;
pub(crate) mod fuzzy;

pub use diagnostics::{Diagnostic, DiagnosticMessage, Help, Severity};

/// Configuration for semantic validation.
#[derive(Clone, Copy)]
pub struct ValidationConfig {
    /// When `true`, unresolved names are reported as errors.
    /// When `false`, they are reported as warnings.
    pub strict_schema: bool,
    /// Maximum Levenshtein distance for "did you mean?" suggestions.
    pub suggestion_threshold: usize,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        ValidationConfig {
            strict_schema: false,
            suggestion_threshold: 2,
        }
    }
}

impl ValidationConfig {
    /// Returns the effective diagnostic severity for unresolved schema names.
    pub fn severity(&self) -> Severity {
        if self.strict_schema {
            Severity::Error
        } else {
            Severity::Warning
        }
    }
}

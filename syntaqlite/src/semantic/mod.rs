// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Semantic analysis: catalog, engine, single-pass analyzer, and rendering.

#[cfg(feature = "validation")]
pub(crate) mod analyzer;
#[cfg(feature = "validation")]
pub(crate) mod catalog;
pub(crate) mod diagnostics;
#[cfg(feature = "validation")]
#[expect(unreachable_pub)]
pub(crate) mod ffi;
#[cfg(feature = "validation")]
pub(crate) mod fuzzy;
#[cfg(feature = "validation")]
pub(crate) mod model;
#[cfg(feature = "validation")]
pub(crate) mod render;

// ── Public re-exports ─────────────────────────────────────────────────────────

#[cfg(feature = "validation")]
pub use analyzer::SemanticAnalyzer;
#[cfg(feature = "validation")]
pub use catalog::Catalog;
pub use diagnostics::{Diagnostic, DiagnosticMessage, Help, Severity};
#[cfg(feature = "validation")]
pub use model::SemanticModel;
#[cfg(feature = "validation")]
pub use render::{DiagnosticRenderer, SourceContext};

/// Whether statements are being analyzed (editing a file) or executed
/// (running sequentially in a session).
///
/// In `Document` mode, DDL from previous [`SemanticAnalyzer::analyze`] calls
/// is discarded — each call analyzes a fresh document.
///
/// In `Execute` mode, DDL accumulates across calls — a `CREATE TABLE` in one
/// call makes the table visible to subsequent calls, matching the behaviour of
/// an interactive database session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnalysisMode {
    /// Statements are being analyzed (e.g. editing a SQL file).
    /// DDL resets between `analyze()` calls.
    #[default]
    Document,
    /// Statements are being executed sequentially.
    /// DDL accumulates across `analyze()` calls.
    Execute,
}

/// Configuration for semantic validation.
#[derive(Clone, Copy)]
pub struct ValidationConfig {
    strict_schema: bool,
    suggestion_threshold: usize,
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
    /// Whether unresolved names are reported as errors (`true`) or warnings (`false`).
    pub fn strict_schema(&self) -> bool {
        self.strict_schema
    }

    /// Maximum Levenshtein distance for "did you mean?" suggestions.
    pub fn suggestion_threshold(&self) -> usize {
        self.suggestion_threshold
    }

    /// Returns the effective diagnostic severity for unresolved schema names.
    pub fn severity(&self) -> Severity {
        if self.strict_schema {
            Severity::Error
        } else {
            Severity::Warning
        }
    }

    /// Set whether unresolved names are reported as errors.
    #[must_use]
    pub fn with_strict_schema(mut self, strict: bool) -> Self {
        self.strict_schema = strict;
        self
    }

    /// Set the maximum Levenshtein distance for suggestions.
    #[must_use]
    pub fn with_suggestion_threshold(mut self, threshold: usize) -> Self {
        self.suggestion_threshold = threshold;
        self
    }
}

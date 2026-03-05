// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Semantic analysis for SQL tooling.
//!
//! [`SemanticAnalyzer`] is the single entry point for all semantic analysis —
//! diagnostics, semantic tokens, completions. It replaces the old `Validator`,
//! `EmbeddedAnalyzer`, and `AnalysisHost`.
//!
//! # Quick start
//!
//! ```
//! use syntaqlite::semantic::{SemanticAnalyzer, DatabaseCatalog, DiagnosticRenderer};
//!
//! let catalog = DatabaseCatalog::default();
//! let mut analyzer = SemanticAnalyzer::new();
//! let diags = analyzer.diagnostics("SELECT 1", &catalog);
//! for diag in &diags {
//!     DiagnosticRenderer::new("SELECT 1", "<stdin>").render_diagnostics(&diags);
//! }
//! ```

// ── Public types ─────────────────────────────────────────────────────

pub(crate) mod analyzer;
pub(crate) mod catalog;
pub(crate) mod diagnostics;
pub(crate) mod fuzzy;
pub(crate) mod model;
pub(crate) mod render;

pub(crate) mod checks;
pub(crate) mod scope;
pub(crate) mod walker;

pub(crate) mod functions;
pub(crate) mod relations;

// ── Re-exports ───────────────────────────────────────────────────────

pub(crate) use analyzer::SemanticAnalyzer;
pub(crate) use catalog::DatabaseCatalog;
pub(crate) use diagnostics::{Diagnostic, DiagnosticMessage, Help, Severity};
pub(crate) use model::{CompletionContext, CompletionInfo, SemanticModel, SemanticToken};
pub(crate) use render::{DiagnosticRenderer, SourceContext};

// Re-export key types for callers.
pub(crate) use functions::SessionFunction;
pub(crate) use functions::{FunctionCatalog, FunctionCheckResult, FunctionDef, FunctionLookup};
pub(crate) use relations::{ColumnDef, RelationCatalog, RelationDef, RelationKind};

// ── Configuration ────────────────────────────────────────────────────

/// Configuration for semantic validation.
#[derive(Clone, Copy)]
pub(crate) struct ValidationConfig {
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
    pub(crate) fn severity(&self) -> Severity {
        if self.strict_schema {
            Severity::Error
        } else {
            Severity::Warning
        }
    }
}

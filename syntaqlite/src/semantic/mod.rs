// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Semantic analysis and validation.
//!
//! Validates SQL against a known database schema — resolving table, column,
//! and function references and producing structured [`Diagnostic`] values with
//! byte-offset spans and "did you mean?" suggestions.
//!
//! The most commonly used types ([`SemanticAnalyzer`], [`Catalog`],
//! [`CatalogLayer`], [`Diagnostic`], [`Severity`], [`ValidationConfig`]) are
//! re-exported at the crate root. This module also provides:
//!
//! - [`SemanticModel`] — the result of a single analysis pass.
//! - [`DiagnosticMessage`] — structured message variants for pattern matching.
//! - [`Help`] — "did you mean?" suggestion attached to a diagnostic.
//! - [`AnalysisMode`] — document vs. execute mode for DDL accumulation.
//! - [`CatalogLayerContents`] — the data stored in a single catalog layer.
//! - [`AritySpec`], [`FunctionCategory`] — function metadata for catalog
//!   registration.
//!
//! # Example
//!
//! ```
//! use syntaqlite::semantic::{
//!     SemanticAnalyzer, Catalog, CatalogLayer, ValidationConfig,
//!     DiagnosticMessage,
//! };
//!
//! let mut analyzer = SemanticAnalyzer::new();
//! let mut catalog = Catalog::new(syntaqlite::sqlite_dialect());
//! catalog.layer_mut(CatalogLayer::Database)
//!     .insert_table("users", Some(vec!["id".into(), "name".into()]), false);
//!
//! let model = analyzer.analyze(
//!     "SELECT id, nme FROM users",
//!     &catalog,
//!     &ValidationConfig::default(),
//! );
//!
//! // "nme" is close to "name" — expect a diagnostic with a suggestion.
//! assert!(!model.diagnostics().is_empty());
//! ```

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

#[doc(inline)]
#[cfg(feature = "validation")]
pub use analyzer::SemanticAnalyzer;
#[doc(inline)]
#[cfg(feature = "validation")]
pub use catalog::{AritySpec, Catalog, CatalogLayer, CatalogLayerContents, FunctionCategory};
#[doc(inline)]
pub use diagnostics::{Diagnostic, DiagnosticMessage, Help, Severity};
#[doc(inline)]
#[cfg(feature = "validation")]
pub use model::SemanticModel;

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
///
/// Controls how the [`SemanticAnalyzer`] reports unresolved names and
/// generates "did you mean?" suggestions:
///
/// - **`strict_schema`** (`false` by default) — when `false`, unresolved
///   table/column/function names produce [`Severity::Warning`]; when `true`,
///   they produce [`Severity::Error`]. Use strict mode for CI pipelines where
///   schema mismatches should block deployment.
/// - **`suggestion_threshold`** (`2` by default) — maximum Levenshtein
///   distance for "did you mean?" suggestions. Set to `0` to disable
///   suggestions entirely.
///
/// # Example
///
/// ```
/// # use syntaqlite::semantic::Severity;
/// # use syntaqlite::ValidationConfig;
/// // Default: warnings + suggestions within edit distance 2.
/// let config = ValidationConfig::default();
/// assert_eq!(config.severity(), Severity::Warning);
/// assert_eq!(config.suggestion_threshold(), 2);
///
/// // Strict mode for CI: errors + tighter suggestions.
/// let strict = ValidationConfig::default()
///     .with_strict_schema(true)
///     .with_suggestion_threshold(1);
/// assert_eq!(strict.severity(), Severity::Error);
/// ```
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

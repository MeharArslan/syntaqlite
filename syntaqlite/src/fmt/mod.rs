// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! SQL formatter.
//!
//! Formats SQL source text using a bytecode interpreter driven by
//! per-node formatting instructions compiled from `.synq` definitions.
//! Layout selection is handled by a Wadler-style document renderer in
//! [`doc`](self::doc), using `group`/`line`/`softline` primitives.
//! The high-level entry point is [`Formatter`](crate::Formatter);
//! configuration types are re-exported at the crate root as
//! [`FormatConfig`](crate::FormatConfig) and
//! [`KeywordCase`](crate::KeywordCase).

mod comment;
mod doc;
pub(crate) mod formatter;
mod interpret;

// ── Config types (formerly config.rs) ────────────────────────────────────

/// Controls how SQL keywords are cased in formatted output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeywordCase {
    /// Convert keywords to UPPER CASE.
    #[default]
    Upper,
    /// Convert keywords to lower case.
    Lower,
}

/// Configuration for the SQL formatter.
#[derive(Debug, Clone)]
pub struct FormatConfig {
    /// Maximum line width before breaking. Default: 80.
    pub line_width: usize,
    /// Number of spaces per indentation level. Default: 2.
    pub indent_width: usize,
    /// How to case SQL keywords. Default: Upper.
    pub keyword_case: KeywordCase,
    /// Append semicolons after each statement. Default: true.
    pub semicolons: bool,
}

/// An error returned by [`crate::Formatter::format`] when a statement fails to parse.
#[derive(Debug, Clone)]
pub struct FormatError {
    /// Human-readable error message.
    pub message: String,
    /// Byte offset of the error token in the source, if known.
    pub offset: Option<usize>,
    /// Byte length of the error token, if known.
    pub length: Option<usize>,
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for FormatError {}

impl Default for FormatConfig {
    fn default() -> Self {
        FormatConfig {
            line_width: 80,
            indent_width: 2,
            keyword_case: KeywordCase::Upper,
            semicolons: true,
        }
    }
}

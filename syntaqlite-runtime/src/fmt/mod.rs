// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#[doc(hidden)]
pub mod bytecode;
mod doc;
mod formatter;
mod interpret;
mod render;
mod comment;

// ── Config types (formerly config.rs) ────────────────────────────────────

/// Controls how SQL keywords are cased in formatted output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordCase {
    /// Keep keywords as they appeared in the source.
    Preserve,
    /// Convert keywords to UPPER CASE.
    Upper,
    /// Convert keywords to lower case.
    Lower,
}

impl Default for KeywordCase {
    fn default() -> Self {
        KeywordCase::Preserve
    }
}

/// Configuration for the SQL formatter.
#[derive(Debug, Clone)]
pub struct FormatConfig {
    /// Maximum line width before breaking. Default: 80.
    pub line_width: usize,
    /// Number of spaces per indentation level. Default: 2.
    pub indent_width: usize,
    /// How to case SQL keywords. Default: Preserve.
    pub keyword_case: KeywordCase,
}

impl Default for FormatConfig {
    fn default() -> Self {
        FormatConfig {
            line_width: 80,
            indent_width: 2,
            keyword_case: KeywordCase::Preserve,
        }
    }
}

// ── Primary public API ─────────────────────────────────────────────────
pub use formatter::Formatter;
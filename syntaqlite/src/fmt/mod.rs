// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! SQL formatter.
//!
//! Formats SQL source text using a bytecode interpreter driven by
//! per-node formatting instructions compiled from `.synq` definitions.
//! The high-level entry point is [`Formatter`](crate::Formatter); this
//! module also exposes [`FormatConfig`] and [`KeywordCase`] for
//! controlling output style.

#[doc(hidden)]
pub mod bytecode;
mod comment;
mod doc;
pub(crate) mod formatter;
mod interpret;

// ── Config types (formerly config.rs) ────────────────────────────────────

/// Controls how SQL keywords are cased in formatted output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeywordCase {
    /// Keep keywords as they appeared in the source.
    #[default]
    Preserve,
    /// Convert keywords to UPPER CASE.
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
    /// How to case SQL keywords. Default: Preserve.
    pub keyword_case: KeywordCase,
    /// Append semicolons after each statement. Default: true.
    pub semicolons: bool,
}

impl Default for FormatConfig {
    fn default() -> Self {
        FormatConfig {
            line_width: 80,
            indent_width: 2,
            keyword_case: KeywordCase::Preserve,
            semicolons: true,
        }
    }
}

impl FormatConfig {
    /// Build a `FormatConfig` from raw integer parameters, as used by FFI/WASM callers.
    ///
    /// - `line_width`: 0 → default 80.
    /// - `keyword_case`: 1 → Upper, 2 → Lower, anything else → Preserve.
    /// - `semicolons`: 0 → false, anything else → true.
    pub fn from_raw_params(line_width: u32, keyword_case: u32, semicolons: u32) -> Self {
        FormatConfig {
            line_width: if line_width == 0 {
                80
            } else {
                line_width as usize
            },
            keyword_case: match keyword_case {
                1 => KeywordCase::Upper,
                2 => KeywordCase::Lower,
                _ => KeywordCase::Preserve,
            },
            semicolons: semicolons != 0,
            ..Default::default()
        }
    }
}

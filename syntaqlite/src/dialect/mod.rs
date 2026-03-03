// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! TypedDialectEnv handle and token classification.
//!
//! A `TypedDialectEnv` is an opaque, `Copy` handle wrapping a pointer to a C
//! dialect descriptor produced by codegen. It provides metadata about
//! node names, field layouts, token categories, keyword tables, and
//! formatter bytecode — everything a parser, formatter, or validator needs
//! to operate on a particular SQL grammar.
//!
//! Most users will never construct a `TypedDialectEnv` directly; the built-in
//! SQLite dialect is available via [`sqlite()`].
//! External dialect crates obtain their handle through the generated
//! dialect descriptor.

// ── Token category ─────────────────────────────────────────────────────

use syntaqlite_parser::DialectEnv;

// ── TypedDialectEnv-generic typed wrappers ──────────────────────────────────────
//
// Re-exported from the internal `parser::typed` module so that dialect
// authors can reach them as `syntaqlite::dialect::DialectParser`, etc.

pub use crate::parser::typed::{
    DialectIncrementalCursor, DialectIncrementalParser, DialectParser, DialectStatementCursor,
    DialectToken, DialectTokenCursor, DialectTokenizer,
};

pub use syntaqlite_parser::{NodeFamily, TypedDialectEnv};

/// Semantic category for a token type, used for syntax highlighting.
///
/// Discriminant values match the corresponding index in [`SEMANTIC_TOKEN_LEGEND`],
/// so `self as u32` directly gives the legend index for non-`Other` variants.
/// `Other` (discriminant 10) is not in the legend and is never emitted as a token.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCategory {
    Keyword = 0,
    Variable = 1,
    String = 2,
    Number = 3,
    Operator = 4,
    Comment = 5,
    Punctuation = 6,
    Identifier = 7,
    Function = 8,
    Type = 9,
    Other = 10,
}

/// The semantic token legend: LSP/Monaco token type names in legend-index order.
///
/// This is the single source of truth for the legend. Both the LSP server
/// capabilities and the WASM/Monaco provider must use this same ordering.
pub const SEMANTIC_TOKEN_LEGEND: &[&str] = &[
    "keyword",     // 0
    "variable",    // 1
    "string",      // 2
    "number",      // 3
    "operator",    // 4
    "comment",     // 5
    "punctuation", // 6
    "identifier",  // 7
    "function",    // 8
    "type",        // 9
];

impl TokenCategory {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Keyword,
            2 => Self::Identifier,
            3 => Self::String,
            4 => Self::Number,
            5 => Self::Operator,
            6 => Self::Punctuation,
            7 => Self::Comment,
            8 => Self::Variable,
            9 => Self::Function,
            10 => Self::Type,
            _ => Self::Other,
        }
    }

    /// The LSP semantic token type name for this category.
    /// Returns `None` for `Other` (not emitted as a semantic token).
    pub fn legend_name(self) -> Option<&'static str> {
        if self == Self::Other {
            None
        } else {
            Some(SEMANTIC_TOKEN_LEGEND[self as usize])
        }
    }
}

/// Extension methods on `TypedDialectEnv` that return typed [`TokenCategory`] values.
///
/// This trait bridges `syntaqlite-parser`'s `TypedDialectEnv` (which returns raw `u8`
/// for crate-boundary reasons) to `syntaqlite`'s `TokenCategory` enum.
/// Import this trait to call [`classify_token`](DialectExt::classify_token) and
/// [`token_category`](DialectExt::token_category) directly on a `TypedDialectEnv`.
pub trait DialectExt {
    /// Classify a token using its type and parser-assigned flags.
    ///
    /// The parser annotates tokens with flags (e.g. `TOKEN_FLAG_AS_FUNCTION`)
    /// when grammar actions identify a keyword used as a function name, type
    /// name, or plain identifier. This checks those flags first, then falls
    /// back to the static per-token-type category table.
    fn classify_token(&self, token_type: u32, flags: u32) -> TokenCategory;

    /// Return the static [`TokenCategory`] for a token type ordinal, ignoring parser flags.
    fn token_category(&self, token_type: u32) -> TokenCategory;
}

impl DialectExt for syntaqlite_parser::DialectEnv<'_> {
    fn classify_token(&self, token_type: u32, flags: u32) -> TokenCategory {
        TokenCategory::from_u8(self.classify_token_raw(token_type, flags))
    }

    fn token_category(&self, token_type: u32) -> TokenCategory {
        TokenCategory::from_u8(self.token_category_raw(token_type))
    }
}

/// Return the built-in SQLite dialect handle.
#[cfg(feature = "sqlite")]
pub fn sqlite() -> DialectEnv<'static> {
    syntaqlite_parser_sqlite::dialect()
}

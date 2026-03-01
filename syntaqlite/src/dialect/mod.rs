// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Dialect handle and token classification.
//!
//! A [`Dialect`] is an opaque, `Copy` handle wrapping a pointer to a C
//! dialect descriptor produced by codegen. It provides metadata about
//! node names, field layouts, token categories, keyword tables, and
//! formatter bytecode — everything a parser, formatter, or validator needs
//! to operate on a particular SQL grammar.
//!
//! Most users will never construct a `Dialect` directly; the built-in
//! SQLite dialect is available via [`sqlite()`].
//! External dialect crates obtain their handle through the generated
//! [`crate::raw::Dialect`] handle.

pub use syntaqlite_parser::dialect::ffi::{CflagInfo, Cflags, DialectConfig, FieldMeta};

// Re-export Dialect, schema types, and field extraction from the sys crate.
pub(crate) use syntaqlite_parser::dialect::extract_fields;
pub use syntaqlite_parser::dialect::{Dialect, SchemaContribution, SchemaKind};

#[cfg(feature = "sqlite")]
pub use syntaqlite_parser::sqlite::{
    cflag_names, cflag_table, parse_cflag_name, parse_sqlite_version,
};

// ── Token category ─────────────────────────────────────────────────────

/// Semantic category for a token type, used for syntax highlighting.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCategory {
    Other = 0,
    Keyword = 1,
    Identifier = 2,
    String = 3,
    Number = 4,
    Operator = 5,
    Punctuation = 6,
    Comment = 7,
    Variable = 8,
    Function = 9,
    Type = 10,
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
        let idx = self.legend_index()?;
        Some(SEMANTIC_TOKEN_LEGEND[idx as usize])
    }

    /// Index into [`SEMANTIC_TOKEN_LEGEND`] for this category.
    /// Returns `None` for `Other`.
    pub fn legend_index(self) -> Option<u32> {
        match self {
            Self::Keyword => Some(0),
            Self::Variable => Some(1),
            Self::String => Some(2),
            Self::Number => Some(3),
            Self::Operator => Some(4),
            Self::Comment => Some(5),
            Self::Punctuation => Some(6),
            Self::Identifier => Some(7),
            Self::Function => Some(8),
            Self::Type => Some(9),
            Self::Other => None,
        }
    }
}

/// Return the built-in SQLite dialect handle.
#[cfg(feature = "sqlite")]
pub fn sqlite() -> &'static Dialect<'static> {
    &crate::sqlite::DIALECT
}

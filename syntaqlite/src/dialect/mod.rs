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
//! [`crate::ext::Dialect`] handle.

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

/// Return the built-in SQLite dialect handle.
#[cfg(feature = "sqlite")]
pub fn sqlite() -> &'static Dialect<'static> {
    &crate::sqlite::DIALECT
}

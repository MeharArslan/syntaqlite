// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

mod ffi;
pub mod ast;
mod wrappers;

use std::sync::LazyLock;

use syntaqlite_runtime::dialect::ffi as dialect_ffi;
unsafe extern "C" {
    // SAFETY: The generated code must provide this function, and it must return
    // a valid pointer to a `dialect_ffi::Dialect` struct with `'static` lifetime.
    fn syntaqlite_sqlite_dialect() -> *const dialect_ffi::Dialect;
}

static DIALECT: LazyLock<syntaqlite_runtime::Dialect<'static>> =
    LazyLock::new(|| unsafe { syntaqlite_runtime::Dialect::from_raw(syntaqlite_sqlite_dialect()) });

// ── Re-exports ─────────────────────────────────────────────────────────

pub use syntaqlite_runtime::NodeId;

/// Low-level APIs for advanced use cases (e.g. custom token feeding/tokenizing).
pub mod low_level {
    pub use crate::wrappers::{TokenFeeder, TokenParser, Tokenizer, TokenCursor};
    pub use crate::tokens::TokenType;

    /// Marker type for the SQLite dialect.
    pub struct Sqlite;

    impl Sqlite {
        /// Access the SQLite dialect (lazy-initialized).
        pub fn dialect() -> &'static syntaqlite_runtime::Dialect<'static> {
            &crate::DIALECT
        }
    }
}

/// Access the SQLite dialect handle (for use with `syntaqlite_runtime` APIs).
pub fn dialect() -> &'static syntaqlite_runtime::Dialect<'static> {
    &DIALECT
}

pub use wrappers::{Formatter, Parser, StatementCursor};
pub use syntaqlite_runtime::ParseError;
pub use syntaqlite_runtime::fmt::{FormatConfig, KeywordCase};

mod tokens;
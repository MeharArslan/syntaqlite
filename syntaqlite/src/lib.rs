mod generated;
mod wrappers;

use std::sync::LazyLock;

use syntaqlite_runtime::dialect::ffi;
unsafe extern "C" {
    // SAFETY: The generated code must provide this function, and it must return
    // a valid pointer to a `ffi::Dialect` struct with `'static` lifetime.
    fn syntaqlite_sqlite_dialect() -> *const ffi::Dialect;
}

static DIALECT: LazyLock<syntaqlite_runtime::Dialect<'static>> =
    LazyLock::new(|| unsafe { syntaqlite_runtime::Dialect::from_raw(syntaqlite_sqlite_dialect()) });

// ── Re-exports ─────────────────────────────────────────────────────────

pub mod ast {
    pub use crate::generated::nodes::*;
    pub use syntaqlite_runtime::{NodeId, NodeList, SourceSpan, Trivia, TriviaKind};
}

/// Low-level APIs for advanced use cases (e.g. custom token feeding/tokenizing).
pub mod low_level {
    pub use crate::wrappers::{TokenFeeder, TokenParser, Tokenizer, TokenCursor};

    /// Marker type for the SQLite dialect.
    pub struct Sqlite;

    impl Sqlite {
        /// Access the SQLite dialect (lazy-initialized).
        pub fn dialect() -> &'static syntaqlite_runtime::Dialect<'static> {
            &crate::DIALECT
        }
    }
}

pub use generated::tokens;
pub use wrappers::{Formatter, Parser, StatementCursor};
pub use syntaqlite_runtime::ParseError;
pub use syntaqlite_runtime::fmt::{FormatConfig, KeywordCase};

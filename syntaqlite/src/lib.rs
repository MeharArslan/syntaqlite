mod generated;

use std::sync::LazyLock;

use generated::nodes::Node;
use generated::tokens::TokenType;
use syntaqlite_runtime::dialect::RawSyntaqliteDialect;

/// Marker type for the SQLite dialect.
pub struct Sqlite;

unsafe extern "C" {
    fn syntaqlite_sqlite_dialect() -> *const RawSyntaqliteDialect;
}

static DIALECT: LazyLock<syntaqlite_runtime::Dialect> = LazyLock::new(|| {
    let raw = unsafe { syntaqlite_sqlite_dialect() };
    assert!(!raw.is_null());
    unsafe { syntaqlite_runtime::Dialect::from_raw(raw as *const std::ffi::c_void) }
});

impl syntaqlite_runtime::DialectTypes for Sqlite {
    type Node<'a> = Node<'a>;
    type TokenType = TokenType;

    unsafe fn node_from_raw<'a>(ptr: *const u32) -> Node<'a> {
        unsafe { Node::from_raw(ptr) }
    }
}

// ── Public API ──────────────────────────────────────────────────────────

/// Access the SQLite dialect (lazy-initialized).
pub fn dialect() -> &'static syntaqlite_runtime::Dialect {
    &DIALECT
}

/// Create a parser pre-configured for the SQLite dialect.
pub fn create_parser() -> syntaqlite_runtime::Parser {
    syntaqlite_runtime::Parser::new(&DIALECT)
}

// ── Re-exports ─────────────────────────────────────────────────────────

pub mod ast {
    pub use crate::generated::nodes::*;
    pub use syntaqlite_runtime::{MacroRegion, NodeList, Trivia, TriviaKind};

    /// Convenience trait that hardcodes `Sqlite` so callers don't need
    /// turbofish: `session.feed(TokenType, text)` just works.
    pub trait SessionExt<'a> {
        fn node(&self, id: syntaqlite_runtime::NodeId) -> Option<Node<'a>>;
        fn feed(
            &mut self,
            token_type: crate::generated::tokens::TokenType,
            text: &str,
        ) -> Result<Option<syntaqlite_runtime::NodeId>, syntaqlite_runtime::ParseError>;
    }

    impl<'a> SessionExt<'a> for syntaqlite_runtime::Session<'a> {
        fn node(&self, id: syntaqlite_runtime::NodeId) -> Option<Node<'a>> {
            <Self as syntaqlite_runtime::SessionExt<'a, crate::Sqlite>>::node(self, id)
        }

        fn feed(
            &mut self,
            token_type: crate::generated::tokens::TokenType,
            text: &str,
        ) -> Result<Option<syntaqlite_runtime::NodeId>, syntaqlite_runtime::ParseError> {
            <Self as syntaqlite_runtime::SessionExt<'a, crate::Sqlite>>::feed(self, token_type, text)
        }
    }
}

pub use generated::tokens;
pub use syntaqlite_runtime::{Dialect, DialectTypes, NodeId, ParseError, Parser, Session, SourceSpan};

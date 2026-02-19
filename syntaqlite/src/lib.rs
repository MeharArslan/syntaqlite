mod generated;

use std::sync::LazyLock;

use generated::nodes::Node;
use generated::tokens::TokenType;
use syntaqlite_runtime::c_dialect::RawSyntaqliteDialect;
use syntaqlite_runtime::ConvertedDialect;

/// Marker type for the SQLite dialect.
pub struct Sqlite;

unsafe extern "C" {
    fn syntaqlite_sqlite_dialect() -> *const RawSyntaqliteDialect;
}

static DIALECT: LazyLock<ConvertedDialect> = LazyLock::new(|| {
    let raw = unsafe { syntaqlite_sqlite_dialect() };
    assert!(!raw.is_null());
    unsafe { syntaqlite_runtime::c_dialect::convert(raw) }
});

impl syntaqlite_runtime::DialectTypes for Sqlite {
    type Node<'a> = Node<'a>;
    type TokenType = TokenType;

    unsafe fn node_from_raw<'a>(ptr: *const u32) -> Node<'a> {
        unsafe { Node::from_raw(ptr) }
    }
}

// ── Public API ──────────────────────────────────────────────────────────

/// Access the fully-converted SQLite dialect (lazy-initialized).
pub fn dialect() -> &'static ConvertedDialect {
    &DIALECT
}

/// Create a parser pre-configured for the SQLite dialect.
pub fn create_parser() -> syntaqlite_runtime::Parser {
    DIALECT.parser()
}

// ── Re-exports ─────────────────────────────────────────────────────────

// AST types & inspection
pub mod ast {
    pub use crate::generated::nodes::*;
    pub use syntaqlite_runtime::{MacroRegion, NodeList, Trivia, TriviaKind};

    /// Convenience trait that hardcodes `Sqlite` so callers don't need
    /// turbofish: `session.node(id)` just works.
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

// Tokens
pub use generated::tokens;

// Parser
pub use syntaqlite_runtime::Parser;

// Runtime types
pub use syntaqlite_runtime::{NodeId, ParseError, Session, SourceSpan};

// Dialect traits
pub use syntaqlite_runtime::DialectTypes;

mod generated;

use std::ffi::c_void;
use std::sync::LazyLock;

use generated::nodes::{Node, NodeTag, FIELD_DESCRIPTORS, NODE_NAMES};
use generated::tokens::TokenType;
use syntaqlite_runtime::Dialect;

#[cfg(feature = "fmt")]
use syntaqlite_runtime::fmt::{LoadedFmt, StaticFmt};

/// Marker type for the SQLite dialect.
pub struct Sqlite;

unsafe extern "C" {
    fn syntaqlite_sqlite_dialect() -> *const c_void;
}

fn is_list_tag(tag: u32) -> bool {
    NodeTag::from_raw(tag).map_or(false, |t| t.is_list())
}

#[cfg(feature = "fmt")]
fn load_fmt() -> StaticFmt {
    LoadedFmt::load(include_bytes!("generated/fmt.bin"))
        .expect("failed to load embedded fmt bytecode")
        .into_static()
}

static INFO: LazyLock<syntaqlite_runtime::DialectInfo> = LazyLock::new(|| {
    let raw = unsafe { syntaqlite_sqlite_dialect() };
    assert!(!raw.is_null());
    let dialect = unsafe { Dialect::from_raw(raw) };
    syntaqlite_runtime::DialectInfo {
        dialect,
        field_descriptors: FIELD_DESCRIPTORS,
        node_names: NODE_NAMES,
        is_list: is_list_tag,
        #[cfg(feature = "fmt")]
        fmt: load_fmt(),
    }
});

impl syntaqlite_runtime::DialectTypes for Sqlite {
    type Node<'a> = Node<'a>;
    type TokenType = TokenType;

    unsafe fn node_from_raw<'a>(ptr: *const u32) -> Node<'a> {
        unsafe { Node::from_raw(ptr) }
    }

    fn info() -> &'static syntaqlite_runtime::DialectInfo {
        &INFO
    }
}

// ── Re-exports ─────────────────────────────────────────────────────────

// AST types & inspection
#[cfg(feature = "parser")]
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
#[cfg(feature = "parser")]
pub use generated::tokens;

// Parser
#[cfg(feature = "parser")]
pub use syntaqlite_runtime::Parser;

/// Create a parser pre-configured for the SQLite dialect.
#[cfg(feature = "parser")]
pub fn create_parser() -> syntaqlite_runtime::Parser {
    INFO.parser()
}

// Runtime types
#[cfg(feature = "parser")]
pub use syntaqlite_runtime::{NodeId, ParseError, Session, SourceSpan};

// The dialect info (everything a consumer needs)
pub use syntaqlite_runtime::{DialectInfo, DialectTypes};

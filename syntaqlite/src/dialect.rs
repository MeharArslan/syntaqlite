use std::ffi::c_void;
use std::sync::LazyLock;

use syntaqlite_runtime::parser::nodes::NodeId;
use syntaqlite_runtime::{Dialect, ParseError, Session};

use crate::generated::nodes::{Node, FIELD_DESCRIPTORS, NODE_NAMES};
use crate::generated::tokens::TokenType;

unsafe extern "C" {
    fn syntaqlite_sqlite_dialect() -> *const c_void;
}

static SQLITE_DIALECT: LazyLock<Dialect> = LazyLock::new(|| {
    let raw = unsafe { syntaqlite_sqlite_dialect() };
    assert!(!raw.is_null());
    unsafe { Dialect::from_raw(raw) }
});

/// Return the SQLite dialect, for use with `Parser::new`.
pub fn sqlite_dialect() -> &'static Dialect {
    &SQLITE_DIALECT
}

/// Extension trait adding typed node access to Session.
pub trait SessionExt<'a> {
    fn node(&self, id: NodeId) -> Option<Node<'a>>;
    fn feed(&mut self, token_type: TokenType, text: &str)
        -> Result<Option<NodeId>, ParseError>;
}

impl<'a> SessionExt<'a> for Session<'a> {
    fn node(&self, id: NodeId) -> Option<Node<'a>> {
        let (ptr, _tag) = self.node_ptr(id)?;
        Some(unsafe { Node::from_raw(ptr as *const u32) })
    }

    fn feed(&mut self, token_type: TokenType, text: &str)
        -> Result<Option<NodeId>, ParseError>
    {
        self.feed_token(token_type as u32, text)
    }
}

/// Dump an AST tree using SQLite node definitions.
pub fn dump_node(session: &Session<'_>, id: NodeId, out: &mut String, indent: usize) {
    syntaqlite_runtime::parser::nodes::dump_node_with(
        &|nid| session.node_ptr(nid),
        session.source(),
        FIELD_DESCRIPTORS,
        NODE_NAMES,
        id,
        out,
        indent,
    )
}

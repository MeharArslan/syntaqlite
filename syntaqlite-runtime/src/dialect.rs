#[cfg(feature = "fmt")]
use crate::fmt::{FmtCtx, NodeFmt, NodeInfo, StaticFmt};
use crate::parser::nodes::{dump_node_with, FieldDescriptor, NodeId};
use crate::parser::{Dialect, ParseError, Parser, Session};

/// All dialect-specific data needed by the runtime.
///
/// Dialect crates construct one `DialectInfo` (typically in a `LazyLock`) and
/// expose it via `DialectTypes::info()`. The CLI, formatter, and AST dumper
/// all pull what they need from this single struct.
pub struct DialectInfo {
    pub dialect: Dialect,
    pub field_descriptors: &'static [&'static [FieldDescriptor]],
    pub node_names: &'static [&'static str],
    pub is_list: fn(u32) -> bool,
    #[cfg(feature = "fmt")]
    pub fmt: StaticFmt,
}

impl DialectInfo {
    /// Create a fresh parser for this dialect.
    pub fn parser(&self) -> Parser {
        Parser::new(&self.dialect)
    }

    /// Dump an AST tree to a string for debugging / the `ast` CLI command.
    pub fn dump_node(&self, session: &Session<'_>, id: NodeId, out: &mut String, indent: usize) {
        dump_node_with(
            &|nid| session.node_ptr(nid),
            session.source(),
            self.field_descriptors,
            self.node_names,
            id,
            out,
            indent,
        )
    }

    /// Build the `NodeInfo` the formatter needs.
    #[cfg(feature = "fmt")]
    pub fn node_info(&self) -> NodeInfo {
        NodeInfo {
            field_descriptors: self.field_descriptors,
            is_list: self.is_list,
        }
    }

    /// Formatter dispatch table.
    #[cfg(feature = "fmt")]
    pub fn dispatch(&self) -> &[Option<NodeFmt>] {
        self.fmt.dispatch
    }

    /// Formatter context (keyword maps, enum display tables, etc.).
    #[cfg(feature = "fmt")]
    pub fn ctx(&self) -> &FmtCtx<'_> {
        &self.fmt.ctx
    }
}

/// Trait that dialect crates implement to provide typed access to nodes and tokens.
///
/// The runtime provides blanket impls of `SessionExt` for any `D: DialectTypes`,
/// so dialect crates don't need to write any session-extension boilerplate.
pub trait DialectTypes: 'static {
    type Node<'a>;
    type TokenType: Copy + Into<u32>;

    /// # Safety
    /// The pointer must point to a valid node struct within the session's arena.
    unsafe fn node_from_raw<'a>(ptr: *const u32) -> Self::Node<'a>;
    fn info() -> &'static DialectInfo;
}

/// Extension trait adding typed node access and token feeding to `Session`.
///
/// Implemented via blanket impl for any `D: DialectTypes`.
pub trait SessionExt<'a, D: DialectTypes> {
    fn node(&self, id: NodeId) -> Option<D::Node<'a>>;
    fn feed(
        &mut self,
        token_type: D::TokenType,
        text: &str,
    ) -> Result<Option<NodeId>, ParseError>;
}

impl<'a, D: DialectTypes> SessionExt<'a, D> for Session<'a> {
    fn node(&self, id: NodeId) -> Option<D::Node<'a>> {
        let (ptr, _tag) = self.node_ptr(id)?;
        Some(unsafe { D::node_from_raw(ptr as *const u32) })
    }

    fn feed(
        &mut self,
        token_type: D::TokenType,
        text: &str,
    ) -> Result<Option<NodeId>, ParseError> {
        self.feed_token(token_type.into(), text)
    }
}

use crate::parser::nodes::NodeId;
use crate::parser::{ParseError, Session};

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

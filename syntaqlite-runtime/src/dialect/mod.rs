//! Dialect types: the opaque handle, C ABI mirror structs, and typed access traits.

pub mod ffi;

use crate::parser::nodes::NodeId;
use crate::parser::{ParseError, Session};

// ── Opaque dialect handle ──────────────────────────────────────────────

/// An opaque dialect handle. Dialect crates (e.g. `syntaqlite`) provide a
/// function that returns a `&'static Dialect` for their grammar.
pub struct Dialect {
    pub(crate) raw: &'static ffi::SyntaqliteDialect,
}

impl Dialect {
    /// Create a `Dialect` from a raw C pointer returned by a dialect's
    /// FFI function (e.g. `syntaqlite_sqlite_dialect`).
    ///
    /// # Safety
    /// The pointer must point to a valid `SyntaqliteDialect` with `'static` lifetime.
    pub unsafe fn from_raw(raw: *const ffi::SyntaqliteDialect) -> Self {
        unsafe { Dialect { raw: &*raw } }
    }
}

// SAFETY: The dialect is a reference to a static C struct with no mutable state.
unsafe impl Send for Dialect {}
unsafe impl Sync for Dialect {}

// ── Typed access traits ────────────────────────────────────────────────

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
    fn feed(&mut self, token_type: D::TokenType, text: &str) -> Result<Option<NodeId>, ParseError>;
}

impl<'a, D: DialectTypes> SessionExt<'a, D> for Session<'a> {
    fn node(&self, id: NodeId) -> Option<D::Node<'a>> {
        let (ptr, _tag) = self.node_ptr(id)?;
        Some(unsafe { D::node_from_raw(ptr as *const u32) })
    }

    fn feed(&mut self, token_type: D::TokenType, text: &str) -> Result<Option<NodeId>, ParseError> {
        self.feed_token(token_type.into(), text)
    }
}

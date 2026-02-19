//! Dialect types: the opaque handle, C ABI mirror structs, and typed access traits.

use crate::parser::nodes::NodeId;
use crate::parser::{ParseError, Session};

// ── Opaque dialect handle ──────────────────────────────────────────────

/// An opaque dialect handle. Dialect crates (e.g. `syntaqlite`) provide a
/// function that returns a `&'static Dialect` for their grammar.
pub struct Dialect {
    pub(crate) raw: *const std::ffi::c_void,
}

impl Dialect {
    /// Create a `Dialect` from a raw C pointer returned by a dialect's
    /// FFI function (e.g. `syntaqlite_sqlite_dialect`).
    ///
    /// # Safety
    /// The pointer must point to a valid `SyntaqliteDialect` with `'static` lifetime.
    pub unsafe fn from_raw(raw: *const std::ffi::c_void) -> Self {
        Dialect { raw }
    }
}

// SAFETY: The dialect is a pointer to a static C struct with no mutable state.
unsafe impl Send for Dialect {}
unsafe impl Sync for Dialect {}

// ── C ABI mirror structs ───────────────────────────────────────────────

pub const FIELD_NODE_ID: u8 = 0;
pub const FIELD_SPAN: u8 = 1;
pub const FIELD_BOOL: u8 = 2;
pub const FIELD_FLAGS: u8 = 3;
pub const FIELD_ENUM: u8 = 4;

#[repr(C)]
pub struct RawFieldMeta {
    pub offset: u16,
    pub kind: u8,
    pub name: *const std::ffi::c_char,
    pub display: *const *const std::ffi::c_char,
    pub display_count: u8,
}

/// Mirrors the C `SyntaqliteDialect` struct defined in `include/syntaqlite/dialect.h`.
#[repr(C)]
pub struct RawSyntaqliteDialect {
    pub name: *const std::ffi::c_char,

    // Parse tables + reduce actions
    pub tables: *const std::ffi::c_void,
    pub reduce_actions: *const std::ffi::c_void,

    // Range metadata
    pub range_meta: *const std::ffi::c_void,

    // Well-known token IDs
    pub tk_space: i32,
    pub tk_semi: i32,
    pub tk_comment: i32,

    // AST metadata
    pub node_count: u32,
    pub node_names: *const *const std::ffi::c_char,
    pub field_meta: *const *const RawFieldMeta,
    pub field_meta_counts: *const u8,
    pub list_tags: *const u8,

    // Formatter bytecode
    pub fmt_data: *const u8,
    pub fmt_data_len: u32,
}

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

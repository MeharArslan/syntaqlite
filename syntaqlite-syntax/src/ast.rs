// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::marker::PhantomData;

use crate::grammar::AnyGrammar;
use crate::parser::AnyStatementResult;

// ── Public API ───────────────────────────────────────────────────────────────

/// A node type that can be resolved from the parser arena by [`AnyNodeId`].
///
/// Implemented by generated view structs so that generic containers like
/// [`TypedNodeList`] can resolve children without dialect-specific code.
///
/// See also the symmetric [`GrammarTokenType`] for token enums.
pub trait GrammarNodeType<'a>: Sized {
    /// Resolve `id` to `Self`, or `None` if null, invalid, or tag mismatch.
    fn from_result(stmt_result: AnyStatementResult<'a>, id: AnyNodeId) -> Option<Self>;
}

/// A token type that can be resolved from a raw ordinal and converted back.
///
/// Each dialect's token enum implements this to enable generic typed tokenizer
/// and cursor usage.
///
/// See also the symmetric [`GrammarNodeType`] for AST node types.
pub trait GrammarTokenType: Sized + Clone + Copy + std::fmt::Debug + Into<u32> {
    /// Resolve a raw token type ordinal into this dialect's token variant,
    /// or `None` if out of range.
    fn from_token_type(raw: u32) -> Option<Self>;
}

impl GrammarTokenType for u32 {
    fn from_token_type(raw: u32) -> Option<Self> {
        Some(raw)
    }
}

/// A raw arena node index. Identifies a node in the parser arena.
///
/// This is the untyped, lifetime-free handle used inside the engine. Dialect
/// crates expose typed `XxxId` newtypes (e.g. `SelectStmtId`) that implement
/// `TypedNodeId` and convert into this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AnyNodeId(pub(crate) u32);

impl AnyNodeId {
    /// Sentinel value representing a missing/null node.
    pub(crate) const NULL: AnyNodeId = AnyNodeId(0xFFFF_FFFF);

    /// Returns `true` if this is the null sentinel.
    pub fn is_null(&self) -> bool {
        self.0 == Self::NULL.0
    }
}

/// A grammar-agnostic AST node used by [`AnyDialect`].
///
/// Wraps the node's arena ID and parser `stmt_result` so that callers can inspect
/// the node without a grammar-specific type.
#[derive(Clone, Copy)]
pub struct AnyNode<'a> {
    pub(crate) id: AnyNodeId,
    pub(crate) stmt_result: AnyStatementResult<'a>,
}

impl<'a> GrammarNodeType<'a> for AnyNode<'a> {
    fn from_result(stmt_result: AnyStatementResult<'a>, id: AnyNodeId) -> Option<Self> {
        stmt_result.node_ptr(id)?; // validate the node exists
        Some(AnyNode { id, stmt_result })
    }
}

impl std::fmt::Debug for AnyNode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node").field("id", &self.id).finish()
    }
}

impl AnyNode<'_> {
    /// Dump as indented text into `out`.
    pub(crate) fn dump(&self, out: &mut String, indent: usize) {
        self.stmt_result.dump_node(self.id, out, indent);
    }
}

/// A type-erasing dialect for use with [`TypedTokenizer`](crate::tokenizer::TypedTokenizer)
/// and [`TypedParser`](crate::parser::TypedParser) when no specific dialect is
/// needed. Wraps a raw grammar handle directly.
#[derive(Clone, Copy)]
pub struct AnyDialect {
    raw: AnyGrammar,
}

impl AnyDialect {
    pub(crate) fn new(raw: AnyGrammar) -> Self {
        AnyDialect { raw }
    }

    /// Returns the underlying [`AnyGrammar`] by value.
    pub fn into_raw(self) -> AnyGrammar {
        self.raw
    }
}

impl crate::grammar::TypedGrammar for AnyDialect {
    type Node<'a> = AnyNode<'a>;
    type NodeId = AnyNodeId;
    type Token = u32;
    fn raw(&mut self) -> &mut AnyGrammar {
        &mut self.raw
    }
}

/// A typed, read-only view over a node list in the parser arena.
///
/// `G` is the dialect grammar; `T` is the element type — a generated view
/// struct, another [`TypedNodeList`], or a heterogeneous node type.
#[derive(Clone, Copy)]
pub struct TypedNodeList<'a, G: crate::grammar::TypedGrammar, T> {
    raw: &'a RawNodeList,
    stmt_result: AnyStatementResult<'a>,
    id: AnyNodeId,
    _phantom: PhantomData<fn() -> (G, T)>,
}

impl<G: crate::grammar::TypedGrammar, T> std::fmt::Debug for TypedNodeList<'_, G, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedNodeList")
            .field("len", &self.raw.children().len())
            .finish()
    }
}

impl<G: crate::grammar::TypedGrammar, T> TypedNodeList<'_, G, T> {
    /// The arena node ID of this list, as the grammar's typed node ID.
    pub fn node_id(&self) -> G::NodeId {
        G::NodeId::from(self.id)
    }

    /// Number of children.
    pub fn len(&self) -> usize {
        self.raw.children().len()
    }

    /// Whether this list has no children.
    pub fn is_empty(&self) -> bool {
        self.raw.children().is_empty()
    }
}

impl<'a, G: crate::grammar::TypedGrammar, T: GrammarNodeType<'a>> TypedNodeList<'a, G, T> {
    /// Get a child by index, or `None` if out of bounds or unresolvable.
    pub fn get(&self, index: usize) -> Option<T> {
        let id = *self.raw.children().get(index)?;
        T::from_result(self.stmt_result, id)
    }

    /// Iterate over children. Unresolvable IDs are silently skipped.
    pub fn iter(&self) -> impl Iterator<Item = T> + 'a {
        let stmt_result = self.stmt_result;
        let children = self.raw.children();
        children
            .iter()
            .filter_map(move |&id| T::from_result(stmt_result, id))
    }
}

/// A lifetime-free handle to a specific typed AST node.
///
/// Generated as `XxxId` newtypes (e.g. `SelectStmtId`) for each concrete view
/// struct. Can be stored without keeping a parser arena alive.
///
/// Resolve back to a typed view using the arena from a parse session.
pub trait TypedNodeId: Copy + Into<AnyNodeId> {
    /// The typed view produced when this ID is resolved against an arena.
    type Node<'a>: GrammarNodeType<'a>;
}

/// A typed field value extracted from an AST node.
///
/// Returned as elements of [`NodeFields`], which is produced by
/// [`AnyStatementResult::extract_fields`](crate::parser::AnyStatementResult::extract_fields).
#[derive(Clone, Copy, Debug)]
pub enum FieldValue<'a> {
    /// A child node reference.
    NodeId(AnyNodeId),
    /// A source text span — a subslice of the original source string.
    Span(&'a str),
    /// A boolean flag.
    Bool(bool),
    /// A compact bitfield of flags.
    Flags(u8),
    /// An enum discriminant.
    Enum(u32),
}

/// A stack-allocated, indexable collection of [`FieldValue`]s for a single AST node.
///
/// Returned by [`AnyStatementResult::extract_fields`](crate::parser::AnyStatementResult::extract_fields).
/// Supports `fields[idx]` indexing; `FieldValue` is `Copy` so indexing yields an owned copy.
pub struct NodeFields<'a> {
    buf: [std::mem::MaybeUninit<FieldValue<'a>>; 16],
    len: usize,
}

impl<'a> NodeFields<'a> {
    /// Create an empty `NodeFields`.
    pub fn new() -> Self {
        Self {
            buf: [const { std::mem::MaybeUninit::uninit() }; 16],
            len: 0,
        }
    }

    /// Append a field value.
    ///
    /// # Panics
    /// Panics if more than 16 fields are pushed.
    pub(crate) fn push(&mut self, val: FieldValue<'a>) {
        assert!(self.len < 16, "NodeFields overflow: more than 16 fields");
        self.buf[self.len] = std::mem::MaybeUninit::new(val);
        self.len += 1;
    }

    /// Number of fields.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether there are no fields.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for NodeFields<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> std::ops::Index<usize> for NodeFields<'a> {
    type Output = FieldValue<'a>;

    fn index(&self, idx: usize) -> &FieldValue<'a> {
        assert!(
            idx < self.len,
            "field index {} out of bounds (len={})",
            idx,
            self.len
        );
        // SAFETY: buf[..len] are all initialised via `push`.
        unsafe { self.buf[idx].assume_init_ref() }
    }
}

impl std::fmt::Debug for NodeFields<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut list = f.debug_list();
        for i in 0..self.len {
            list.entry(&self[i]);
        }
        list.finish()
    }
}

// ── Crate-internal ───────────────────────────────────────────────────────────

/// Blanket [`GrammarNodeType`] impl for [`TypedNodeList`] — resolves the ID as a list node.
impl<'a, G: crate::grammar::TypedGrammar, T> GrammarNodeType<'a> for TypedNodeList<'a, G, T> {
    fn from_result(stmt_result: AnyStatementResult<'a>, id: AnyNodeId) -> Option<Self> {
        let raw = stmt_result.resolve_list(id)?;
        Some(TypedNodeList {
            raw,
            stmt_result,
            id,
            _phantom: PhantomData,
        })
    }
}

/// A typed field value extracted from a node struct.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum FieldVal<'a> {
    /// Child node or list reference.
    NodeId(AnyNodeId),
    /// Source text slice with its byte offset in the original source.
    Span(&'a str, u32),
    /// Boolean flag.
    Bool(bool),
    /// Raw flags byte.
    Flags(u8),
    /// Enum ordinal.
    Enum(u32),
}

/// Extracted fields of a node. Returned by [`AnyStatementResult::extract_fields`].
///
/// Uses `MaybeUninit` internally so that construction is zero-cost — no need
/// to initialize all 16 slots when most nodes only have 2–5 fields.
#[allow(dead_code)]
pub(crate) struct Fields<'a> {
    buf: [std::mem::MaybeUninit<FieldVal<'a>>; 16],
    len: usize,
}

#[allow(dead_code)]
impl<'a> Fields<'a> {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            buf: [const { std::mem::MaybeUninit::uninit() }; 16],
            len: 0,
        }
    }

    #[inline]
    pub(crate) fn push(&mut self, val: FieldVal<'a>) {
        self.buf[self.len] = std::mem::MaybeUninit::new(val);
        self.len += 1;
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for Fields<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> std::ops::Deref for Fields<'a> {
    type Target = [FieldVal<'a>];
    fn deref(&self) -> &[FieldVal<'a>] {
        // SAFETY: buf[..len] slots were all written via `push`.
        unsafe { std::slice::from_raw_parts(self.buf.as_ptr().cast(), self.len) }
    }
}

/// Implemented by each `#[repr(C)]` arena node struct to declare its type tag.
///
/// # Safety
/// Implementors must guarantee that `TAG` matches the `tag` field value
/// that the C parser writes into the first `u32` of the struct.
pub(crate) unsafe trait ArenaNode {
    const TAG: u32;
}

#[allow(dead_code)]
pub(crate) const FIELD_NODE_ID: u8 = 0;
#[allow(dead_code)]
pub(crate) const FIELD_SPAN: u8 = 1;
#[allow(dead_code)]
pub(crate) const FIELD_BOOL: u8 = 2;
#[allow(dead_code)]
pub(crate) const FIELD_FLAGS: u8 = 3;
#[allow(dead_code)]
pub(crate) const FIELD_ENUM: u8 = 4;

// ── ffi ───────────────────────────────────────────────────────────────────────

pub(crate) use ffi::{CNodeList as RawNodeList, CSourceSpan as SourceSpan};

mod ffi {
    use crate::ast::AnyNodeId;

    /// A source byte range within the parser's source buffer.
    ///
    /// Mirrors the C `SyntaqliteSpan` layout: `offset` and `length` in bytes.
    /// Used in generated node structs for token-valued fields (identifiers, literals).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    #[repr(C)]
    pub(crate) struct CSourceSpan {
        pub(crate) offset: u32,
        pub(crate) length: u16,
    }

    impl CSourceSpan {
        /// Returns `true` if the span covers zero bytes.
        #[allow(dead_code)]
        pub(crate) fn is_empty(self) -> bool {
            self.length == 0
        }

        /// Slice the span out of the given source string.
        pub(crate) fn as_str(self, source: &str) -> &str {
            let start = self.offset as usize;
            let end = start + self.length as usize;
            &source[start..end]
        }
    }

    /// List node header — `tag` + `count`, followed by `count` child [`AnyNodeId`]s
    /// in trailing data. The parser arena guarantees this contiguous layout.
    #[derive(Debug)]
    #[repr(C)]
    pub(crate) struct CNodeList {
        pub(crate) tag: u32,
        pub(crate) count: u32,
    }

    impl CNodeList {
        /// The child node IDs stored after this header in the arena.
        pub(crate) fn children(&self) -> &[AnyNodeId] {
            // SAFETY: The arena allocates list nodes as { tag, count, children[count] }
            // contiguously, so `count` u32 values immediately follow this header.
            // CNodeList is only constructed from valid arena pointers (validated tag).
            // AnyNodeId is #[repr(transparent)] over u32, so &[AnyNodeId] is
            // layout-compatible with &[u32].
            unsafe {
                let base = std::ptr::from_ref::<CNodeList>(self)
                    .add(1)
                    .cast::<AnyNodeId>();
                std::slice::from_raw_parts(base, self.count as usize)
            }
        }
    }
}

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::marker::PhantomData;

use crate::grammar::{FieldMeta, RawGrammar};
use crate::parser::ParseResult;

// ── Crate API ────────────────────────────────────────────────────────────────

// ── GrammarNodeType ───────────────────────────────────────────────────────────

/// A node type that can be resolved from the parser arena by [`RawNodeId`].
///
/// Implemented by generated view structs so that generic containers like
/// [`TypedList`] can resolve children without dialect-specific code.
///
/// See also the symmetric [`GrammarTokenType`] for token enums.
pub(crate) trait GrammarNodeType<'a>: Sized {
    /// Resolve `id` to `Self`, or `None` if null, invalid, or tag mismatch.
    fn from_arena(reader: ParseResult<'a>, id: RawNodeId) -> Option<Self>;
}

// ── GrammarTokenType ──────────────────────────────────────────────────────────

/// A token type that can be resolved from a raw ordinal and converted back.
///
/// Each dialect's token enum implements this to enable generic typed tokenizer
/// and cursor usage.
///
/// See also the symmetric [`GrammarNodeType`] for AST node types.
pub(crate) trait GrammarTokenType:
    Sized + Clone + Copy + std::fmt::Debug + Into<u32>
{
    /// Resolve a raw token type ordinal into this dialect's token variant,
    /// or `None` if out of range.
    fn from_token_type(raw: u32) -> Option<Self>;
}

impl GrammarTokenType for u32 {
    fn from_token_type(raw: u32) -> Option<Self> {
        Some(raw)
    }
}

// ── NodeId ────────────────────────────────────────────────────────────────────

/// A lifetime-free handle to a specific typed AST node.
///
/// Generated as `XxxId` newtypes (e.g. `SelectStmtId`) for each concrete view
/// struct. Can be stored without keeping a parser arena alive.
///
/// Resolve back to a typed view with
/// [`StatementCursor::node_ref`](crate::parser::StatementCursor::node_ref) or
/// [`IncrementalCursor::node_ref`](crate::incremental::IncrementalCursor::node_ref).
pub(crate) trait NodeId: Copy + Into<RawNodeId> {
    /// The typed view produced when this ID is resolved against an arena.
    type Node<'a>: GrammarNodeType<'a>;
}

// ── Node ──────────────────────────────────────────────────────────────────────

/// A grammar-agnostic handle to a parsed AST node.
///
/// Bundles a node's arena ID with the [`ParseResult`] and [`RawGrammar`]
/// needed to inspect it, enabling ergonomic methods like [`name()`](Self::name),
/// [`children()`](Self::children), and [`dump()`](Self::dump) without threading
/// three arguments everywhere.
#[derive(Clone, Copy)]
pub(crate) struct RawNode<'a> {
    id: RawNodeId,
    result: ParseResult<'a>,
    grammar: RawGrammar,
}

impl std::fmt::Debug for RawNode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node").field("id", &self.id).finish()
    }
}

impl<'a> RawNode<'a> {
    /// Create a `RawNode` from its constituent parts.
    pub(crate) fn new(id: RawNodeId, result: ParseResult<'a>, grammar: RawGrammar) -> Self {
        RawNode {
            id,
            result,
            grammar,
        }
    }

    /// Raw arena ID (escape hatch for FFI/codegen).
    pub(crate) fn id(&self) -> RawNodeId {
        self.id
    }

    /// Reader for typed access via [`GrammarNodeType`].
    pub(crate) fn reader(&self) -> ParseResult<'a> {
        self.result
    }

    /// Grammar handle.
    pub(crate) fn grammar(&self) -> RawGrammar {
        self.grammar
    }

    /// Node type tag, or `None` if null/invalid.
    pub(crate) fn tag(&self) -> Option<u32> {
        self.result.node_tag(self.id)
    }

    /// Node type name (e.g. `"SelectStmt"`), or `""` if null/invalid.
    pub(crate) fn name(&self) -> &str {
        match self.tag() {
            Some(tag) => self.grammar.node_name(tag),
            None => "",
        }
    }

    /// Whether this is a list node.
    pub(crate) fn is_list(&self) -> bool {
        match self.tag() {
            Some(tag) => self.grammar.is_list(tag),
            None => false,
        }
    }

    /// Child nodes: list children (for lists) or node-typed fields (for non-lists).
    ///
    /// Null child IDs are omitted.
    pub(crate) fn children(&self) -> Vec<RawNode<'a>> {
        self.result
            .child_node_ids(self.id, &self.grammar)
            .into_iter()
            .map(|child_id| RawNode {
                id: child_id,
                result: self.result,
                grammar: self.grammar,
            })
            .collect()
    }

    /// Raw list children slice (for list nodes only).
    pub(crate) fn list_children(&self) -> Option<&'a [RawNodeId]> {
        self.result.list_children(self.id, &self.grammar)
    }

    /// Grammar field metadata for this node type.
    pub(crate) fn field_meta(&self) -> &[FieldMeta] {
        match self.tag() {
            Some(tag) => self.grammar.field_meta(tag),
            None => &[],
        }
    }

    /// Extract typed field values for this node.
    pub(crate) fn extract_fields(&self) -> Option<(u32, Fields<'a>)> {
        self.result.extract_fields(self.id, &self.grammar)
    }

    /// Dump as indented text into `out`.
    pub(crate) fn dump(&self, out: &mut String, indent: usize) {
        self.result.dump_node(self.id, out, indent)
    }

    /// Resolve as a typed AST node, or `None` on tag mismatch.
    pub(crate) fn as_typed<T: GrammarNodeType<'a>>(self) -> Option<T> {
        T::from_arena(self.result, self.id)
    }

    /// The source text bound to this node's reader.
    pub(crate) fn source(&self) -> &'a str {
        self.result.source()
    }
}

// ── AnyDialect ────────────────────────────────────────────────────────────────

/// An uninhabited node type used by [`AnyDialect`]. Never constructed.
pub(crate) enum AnyNode {}

impl<'a> GrammarNodeType<'a> for AnyNode {
    fn from_arena(_reader: ParseResult<'a>, _id: RawNodeId) -> Option<Self> {
        None
    }
}

/// A type-erasing dialect for use with [`TypedTokenizer`](crate::tokenizer::TypedTokenizer)
/// when no specific dialect is needed. Wraps a [`RawGrammar`] directly.
#[derive(Clone, Copy)]
pub(crate) struct AnyDialect {
    pub(crate) raw: crate::grammar::RawGrammar,
}

impl crate::grammar::TypedGrammar for AnyDialect {
    type Node<'a> = AnyNode;
    type Token = u32;
    fn raw(&mut self) -> &mut crate::grammar::RawGrammar {
        &mut self.raw
    }
}

// ── TypedList ─────────────────────────────────────────────────────────────────

/// A typed, read-only view over a [`NodeList`] in the parser arena.
///
/// `T` is the element type — a generated view struct, another [`TypedList`],
/// or [`Node<'a>`] for heterogeneous lists.
#[derive(Clone, Copy)]
pub(crate) struct TypedList<'a, T> {
    raw: &'a NodeList,
    reader: ParseResult<'a>,
    id: RawNodeId,
    _phantom: PhantomData<fn() -> T>,
}

impl<T> std::fmt::Debug for TypedList<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedList")
            .field("len", &self.raw.children().len())
            .finish()
    }
}

impl<'a, T> TypedList<'a, T> {
    /// The arena node ID of this list.
    pub(crate) fn node_id(&self) -> RawNodeId {
        self.id
    }

    /// Number of children.
    pub(crate) fn len(&self) -> usize {
        self.raw.children().len()
    }

    /// Whether this list has no children.
    pub(crate) fn is_empty(&self) -> bool {
        self.raw.children().is_empty()
    }
}

impl<'a, T: GrammarNodeType<'a>> TypedList<'a, T> {
    /// Get a child by index, or `None` if out of bounds or unresolvable.
    pub(crate) fn get(&self, index: usize) -> Option<T> {
        let id = *self.raw.children().get(index)?;
        T::from_arena(self.reader, id)
    }

    /// Iterate over children. Unresolvable IDs are silently skipped.
    pub(crate) fn iter(&self) -> impl Iterator<Item = T> + 'a {
        let reader = self.reader;
        let children = self.raw.children();
        children
            .iter()
            .filter_map(move |&id| T::from_arena(reader, id))
    }
}

/// Blanket [`GrammarNodeType`] impl for [`TypedList`] — resolves the ID as a list node.
impl<'a, T> GrammarNodeType<'a> for TypedList<'a, T> {
    fn from_arena(reader: ParseResult<'a>, id: RawNodeId) -> Option<Self> {
        let raw = reader.resolve_list(id)?;
        Some(TypedList {
            raw,
            reader,
            id,
            _phantom: PhantomData,
        })
    }
}

// ── FieldVal / Fields ─────────────────────────────────────────────────────────

/// A typed field value extracted from a node struct.
#[derive(Clone, Copy, Debug)]
pub(crate) enum FieldVal<'a> {
    /// Child node or list reference.
    NodeId(RawNodeId),
    /// Source text slice with its byte offset in the original source.
    Span(&'a str, u32),
    /// Boolean flag.
    Bool(bool),
    /// Raw flags byte.
    Flags(u8),
    /// Enum ordinal.
    Enum(u32),
}

/// Extracted fields of a node. Returned by [`ParseResult::extract_fields`].
///
/// Uses `MaybeUninit` internally so that construction is zero-cost — no need
/// to initialize all 16 slots when most nodes only have 2–5 fields.
pub(crate) struct Fields<'a> {
    buf: [std::mem::MaybeUninit<FieldVal<'a>>; 16],
    len: usize,
}

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

impl<'a> Default for Fields<'a> {
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

// ── ArenaNode ─────────────────────────────────────────────────────────────────

/// Implemented by each `#[repr(C)]` arena node struct to declare its type tag.
///
/// # Safety
/// Implementors must guarantee that `TAG` matches the `tag` field value
/// that the C parser writes into the first `u32` of the struct.
pub(crate) unsafe trait ArenaNode {
    const TAG: u32;
}

// ── Field kind constants ───────────────────────────────────────────────────────

pub(crate) const FIELD_NODE_ID: u8 = 0;
pub(crate) const FIELD_SPAN: u8 = 1;
pub(crate) const FIELD_BOOL: u8 = 2;
pub(crate) const FIELD_FLAGS: u8 = 3;
pub(crate) const FIELD_ENUM: u8 = 4;

// ── ffi ───────────────────────────────────────────────────────────────────────

pub(crate) use ffi::{CNodeList as NodeList, CSourceSpan as SourceSpan, RawNodeId};

mod ffi {
    /// A raw arena node index. Identifies a node in the parser arena.
    ///
    /// This is the untyped, lifetime-free handle used inside the engine. Dialect
    /// crates expose typed `XxxId` newtypes (e.g. `SelectStmtId`) that implement
    /// [`NodeId`] and convert into this.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub(crate) struct RawNodeId(pub(crate) u32);

    impl RawNodeId {
        /// Sentinel value representing a missing/null node.
        pub(crate) const NULL: RawNodeId = RawNodeId(0xFFFF_FFFF);

        /// Returns `true` if this is the null sentinel.
        pub(crate) fn is_null(&self) -> bool {
            self.0 == Self::NULL.0
        }
    }

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
        pub(crate) fn is_empty(&self) -> bool {
            self.length == 0
        }

        /// Slice the span out of the given source string.
        pub(crate) fn as_str<'a>(&self, source: &'a str) -> &'a str {
            let start = self.offset as usize;
            let end = start + self.length as usize;
            &source[start..end]
        }
    }

    /// List node header — `tag` + `count`, followed by `count` child [`RawNodeId`]s
    /// in trailing data. The parser arena guarantees this contiguous layout.
    #[derive(Debug)]
    #[repr(C)]
    pub(crate) struct CNodeList {
        pub(crate) tag: u32,
        pub(crate) count: u32,
    }

    impl CNodeList {
        /// The child node IDs stored after this header in the arena.
        pub(crate) fn children(&self) -> &[RawNodeId] {
            // SAFETY: The arena allocates list nodes as { tag, count, children[count] }
            // contiguously, so `count` u32 values immediately follow this header.
            // CNodeList is only constructed from valid arena pointers (validated tag).
            // RawNodeId is #[repr(transparent)] over u32, so &[RawNodeId] is
            // layout-compatible with &[u32].
            unsafe {
                let base = (self as *const CNodeList).add(1) as *const RawNodeId;
                std::slice::from_raw_parts(base, self.count as usize)
            }
        }
    }
}

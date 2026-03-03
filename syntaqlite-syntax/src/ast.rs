// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

pub(crate) const FIELD_NODE_ID: u8 = 0;
pub(crate) const FIELD_SPAN: u8 = 1;
pub(crate) const FIELD_BOOL: u8 = 2;
pub(crate) const FIELD_FLAGS: u8 = 3;
pub(crate) const FIELD_ENUM: u8 = 4;

/// A raw arena node index. Identifies a node in the parser arena.
///
/// This is the untyped, lifetime-free handle. For typed handles see the
/// `XxxId` newtypes generated for each AST node (e.g. `SelectStmtId`),
/// which implement the [`crate::TypedNodeId`] trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct NodeId(pub u32);

/// A grammar-agnostic handle to a parsed AST node.
///
/// Bundles a node's arena ID with the reader and dialect needed to
/// inspect it, enabling ergonomic methods like `name()`, `children()`,
/// and `dump()` without threading three arguments everywhere.
#[derive(Clone, Copy)]
pub struct Node<'a> {
    id: NodeId,
    result: ParseResult<'a>,
    dialect: DialectEnv<'a>,
}

impl std::fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeRef").field("id", &self.id).finish()
    }
}

impl<'a> Node<'a> {
    /// Create a `NodeRef` from its constituent parts.
    pub(crate) fn new(id: NodeId, result: ParseResult<'a>, dialect: DialectEnv<'a>) -> Self {
        Node {
            id,
            result,
            dialect,
        }
    }

    /// Raw arena ID (escape hatch for FFI/codegen).
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Reader for typed access via `DialectNodeType`.
    pub fn reader(&self) -> ParseResult<'a> {
        self.result
    }

    /// TypedDialectEnv handle.
    pub fn dialect(&self) -> DialectEnv<'a> {
        self.dialect
    }

    /// Node type tag, or `None` if null/invalid.
    pub fn tag(&self) -> Option<u32> {
        self.result.node_tag(self.id)
    }

    /// Node type name (e.g. `"SelectStmt"`).
    pub fn name(&self) -> &str {
        match self.tag() {
            Some(tag) => self.dialect.node_name(tag),
            None => "",
        }
    }

    /// Whether this is a list node.
    pub fn is_list(&self) -> bool {
        match self.tag() {
            Some(tag) => self.dialect.is_list(tag),
            None => false,
        }
    }

    /// Child nodes: list children (for lists) or node-typed fields (for nodes).
    pub fn children(&self) -> Vec<Node<'a>> {
        self.result
            .child_node_ids(self.id, &self.dialect)
            .into_iter()
            .map(|child_id| Node {
                id: child_id,
                result: self.result,
                dialect: self.dialect,
            })
            .collect()
    }

    /// Raw list children slice (for list nodes only).
    pub fn list_children(&self) -> Option<&'a [NodeId]> {
        self.result.list_children(self.id, &self.dialect)
    }

    /// TypedDialectEnv field metadata for this node type.
    pub fn field_meta(&self) -> &[crate::dialect::FieldMeta] {
        match self.tag() {
            Some(tag) => self.dialect.field_meta(tag),
            None => &[],
        }
    }

    /// Extract typed field values.
    pub fn extract_fields(&self) -> Option<(u32, crate::ast::Fields<'a>)> {
        self.result.extract_fields(self.id, &self.dialect)
    }

    /// Dump as indented text.
    pub fn dump(&self, out: &mut String, indent: usize) {
        self.result.dump_node(self.id, out, indent)
    }

    /// Resolve as a typed AST node.
    pub fn as_typed<T: DialectNodeType<'a>>(self) -> Option<T> {
        T::from_arena(self.result, self.id)
    }

    /// The source text bound to this node's reader.
    pub fn source(&self) -> &'a str {
        self.result.source()
    }
}

impl NodeId {
    /// Sentinel value representing a missing/null node.
    pub const NULL: NodeId = NodeId(0xFFFF_FFFF);

    /// Returns `true` if this is the null sentinel.
    pub fn is_null(&self) -> bool {
        self.0 == Self::NULL.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct SourceSpan {
    pub offset: u32,
    pub length: u16,
}

impl SourceSpan {
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn as_str<'a>(&self, source: &'a str) -> &'a str {
        let start = self.offset as usize;
        let end = start + self.length as usize;
        &source[start..end]
    }
}

/// Each `#[repr(C)]` FFI node struct declares its arena tag via this trait.
///
/// # Safety
/// Implementors must guarantee that `TAG` matches the `tag` field value
/// that the C parser writes into the first `u32` of the struct.
pub unsafe trait ArenaNode {
    const TAG: u32;
}

/// List node header — tag + count, followed by `count` child u32 IDs in
/// trailing data. The parser arena guarantees the trailing layout.
#[derive(Debug)]
#[repr(C)]
pub struct NodeList {
    pub(crate) tag: u32,
    pub(crate) count: u32,
}

impl NodeList {
    pub fn children(&self) -> &[NodeId] {
        // SAFETY: The arena allocates list nodes as { tag, count, children[count] }
        // contiguously, so `count` u32 values immediately follow this header.
        // NodeList is only constructed via Node::from_raw() which validates the
        // tag from a valid arena pointer. NodeId is #[repr(transparent)] over u32,
        // so &[NodeId] has the same layout as &[u32].
        unsafe {
            let base = (self as *const NodeList).add(1) as *const NodeId;
            std::slice::from_raw_parts(base, self.count as usize)
        }
    }
}

/// Extracted fields of a node, returned by value from field extraction helpers.
///
/// Uses `MaybeUninit` internally so that `new()` is zero-cost — no need to
/// initialize all 16 slots when most nodes only have 2-5 fields.
pub struct Fields<'a> {
    buf: [std::mem::MaybeUninit<FieldVal<'a>>; 16],
    len: usize,
}

impl<'a> Fields<'a> {
    #[inline]
    pub fn new() -> Self {
        Self {
            buf: [const { std::mem::MaybeUninit::uninit() }; 16],
            len: 0,
        }
    }

    #[inline]
    pub fn push(&mut self, val: FieldVal<'a>) {
        self.buf[self.len] = std::mem::MaybeUninit::new(val);
        self.len += 1;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
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

/// A typed field value extracted from a node struct.
#[derive(Clone, Copy, Debug)]
pub enum FieldVal<'a> {
    /// Node ID (child node or list reference).
    NodeId(NodeId),
    /// Source text from a SourceSpan field, with its source offset.
    Span(&'a str, u32),
    /// Boolean value.
    Bool(bool),
    /// Flags byte.
    Flags(u8),
    /// Enum ordinal.
    Enum(u32),
}

/// A grammar handle tagged with a [`NodeFamily`], carrying both the raw
/// C grammar pointer and the knowledge of which node/token types it produces.
///
/// Use this at construction boundaries (`Parser::new`, etc.) so the
/// node type parameter `N` can be inferred automatically.
///
/// Call [`raw()`](Self::raw) to downgrade to an untyped [`Grammar`] for
/// passing into untyped infrastructure.
#[derive(Clone, Copy)]
pub struct TypedGrammar<N: NodeFamily> {
    inner: Grammar,
    _marker: PhantomData<N>,
}

// SAFETY: same reasoning as Grammar — wraps immutable static C data.
unsafe impl<N: NodeFamily> Send for TypedGrammar<N> {}
unsafe impl<N: NodeFamily> Sync for TypedGrammar<N> {}

impl<N: NodeFamily> TypedGrammar<N> {
    /// Build a `TypedGrammar` from a [`Grammar`] handle.
    pub fn new(grammar: Grammar) -> Self {
        TypedGrammar {
            inner: grammar,
            _marker: PhantomData,
        }
    }

    /// Return the untagged [`Grammar`] handle.
    pub fn raw(&self) -> Grammar<'g> {
        self.inner
    }
}

impl<'g, N: NodeFamily> From<TypedGrammar<'g, N>> for Grammar<'g> {
    fn from(tg: TypedGrammar<'g, N>) -> Self {
        tg.inner
    }
}
/// A typed, read-only view over a `NodeList` in the parser arena.
///
/// `T` is the element type — a concrete view struct, a typed list, or
/// the `Node<'a>` enum for heterogeneous lists.
#[derive(Clone, Copy)]
pub struct TypedList<'a, T> {
    raw: &'a NodeList,
    reader: ParseResult<'a>,
    id: NodeId,
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
    pub fn node_id(&self) -> NodeId {
        self.id
    }

    /// Number of children in this list.
    pub fn len(&self) -> usize {
        self.raw.children().len()
    }

    /// Whether this list is empty.
    pub fn is_empty(&self) -> bool {
        self.raw.children().is_empty()
    }
}

impl<'a, T: GrammarNodeType<'a>> TypedList<'a, T> {
    /// Get a child by index.
    pub fn get(&self, index: usize) -> Option<T> {
        let id = *self.raw.children().get(index)?;
        T::from_arena(self.reader, id)
    }

    /// Iterate over children.
    pub fn iter(&self) -> impl Iterator<Item = T> + 'a {
        let reader = self.reader; // Copy
        let children = self.raw.children();
        children
            .iter()
            .filter_map(move |&id| T::from_arena(reader, id))
    }
}

/// Blanket `GrammarNodeType` for `TypedList` — resolves the `NodeId` as a list node.
impl<'a, T> GrammarNodeType<'a> for TypedList<'a, T> {
    fn from_arena(reader: ParseResult<'a>, id: NodeId) -> Option<Self> {
        let raw = reader.resolve_list(id)?;
        Some(TypedList {
            raw,
            reader,
            id,
            _phantom: PhantomData,
        })
    }
}

/// A node type that can be resolved from the parser arena by [`NodeId`].
///
/// Implemented by generated view structs (node views, `Node` enum) so that
/// generic containers like `TypedList` can resolve children without
/// dialect-specific code.
///
/// See also the symmetric [`GrammarTokenType`] for token enums.
pub trait GrammarNodeType<'a>: Sized {
    fn from_arena(reader: ParseResult<'a>, id: NodeId) -> Option<Self>;
}

/// A token type that can be resolved from a raw token integer, and converted
/// back to one.
///
/// Each dialect's token enum must implement this trait to enable generic typed
/// tokenizer and cursor usage.
///
/// See also the symmetric [`GrammarNodeType`] for AST node types.
pub trait GrammarTokenType: Sized + Clone + Copy + std::fmt::Debug + Into<u32> {
    /// Attempt to resolve a raw token type code into this dialect's token variant.
    fn from_token_type(raw: u32) -> Option<Self>;
}

/// Bundles the node and token types for a dialect into a single type parameter.
///
/// Implementing this trait for a zero-sized marker type (e.g. `SqliteNodeFamily`)
/// allows the tagged [`TypedDialectEnv<'d, N>`](crate::TypedDialectEnv) handle to infer both
/// the node and token types at construction.
pub trait NodeFamily {
    /// The top-level typed AST node (e.g. `Stmt<'a>`).
    type Node<'a>: GrammarNodeType<'a>;
    /// The typed token enum (e.g. `TokenType`).
    type Token: GrammarTokenType;
}

/// A typed node identifier: a lifetime-free handle to a specific AST node.
///
/// Generated as `XxxId` for each concrete view struct (e.g. `SelectStmtId`).
/// Can be stored freely without holding the parser arena alive.
///
/// Use [`cursor.resolve(id)`](crate::StatementCursor::node_ref) to
/// convert back to a typed view when a cursor is available.
pub trait TypedNodeId: Copy + Into<NodeId> {
    /// The typed view produced when this ID is resolved against an arena.
    type Node<'a>: GrammarNodeType<'a>;
}

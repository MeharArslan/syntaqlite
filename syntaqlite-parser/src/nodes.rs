// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

/// A typed wrapper around a raw arena node ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct NodeId(pub u32);

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
    pub tag: u32,
    pub count: u32,
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

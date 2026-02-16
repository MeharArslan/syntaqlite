use std::marker::PhantomData;

use crate::generated::nodes::NodeTag;

pub const NULL_NODE: u32 = 0xFFFF_FFFF;

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
        &source[self.offset as usize..(self.offset as usize + self.length as usize)]
    }
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
    pub fn children(&self) -> &[u32] {
        // SAFETY: The arena allocates list nodes as { tag, count, children[count] }
        // contiguously, so `count` u32 values immediately follow this header.
        // NodeList is only constructed via NodeRef::as_list() which validates the
        // tag, and NodeRef itself is only created from arena pointers.
        unsafe {
            let base = (self as *const NodeList).add(1) as *const u32;
            std::slice::from_raw_parts(base, self.count as usize)
        }
    }
}

/// A reference to a node in the parser arena.
///
/// Core struct and methods are hand-written; the generated `nodes.rs` extends
/// this with `as_*` methods for each node type.
#[derive(Clone, Copy)]
pub struct NodeRef<'a> {
    ptr: *const u32,
    _marker: PhantomData<&'a ()>,
}

impl<'a> std::fmt::Debug for NodeRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeRef").field("tag", &self.tag()).finish()
    }
}

impl<'a> NodeRef<'a> {
    /// Create a NodeRef from a non-null arena pointer.
    ///
    /// # Safety
    /// `ptr` must be non-null, well-aligned, and valid for lifetime `'a`.
    /// It must point to a node in the parser arena whose first u32 is a
    /// valid `NodeTag` discriminant.
    pub(crate) unsafe fn from_raw(ptr: *const u32) -> Self {
        NodeRef {
            ptr,
            _marker: PhantomData,
        }
    }

    /// Read the node's tag.
    pub fn tag(&self) -> NodeTag {
        // SAFETY: `ptr` is a valid arena pointer (guaranteed by from_raw's
        // contract). The first u32 of every arena node is the tag.
        let raw = unsafe { *self.ptr };
        NodeTag::from_raw(raw).unwrap_or(NodeTag::Null)
    }

    /// Access the underlying pointer.
    pub fn ptr(&self) -> *const u32 {
        self.ptr
    }
}

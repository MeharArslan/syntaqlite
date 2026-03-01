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

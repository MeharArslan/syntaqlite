// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::marker::PhantomData;

use crate::dialect_traits::DialectNodeType;
use crate::nodes::{NodeId, NodeList};
use crate::session::ParseResult;

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

impl<'a, T: DialectNodeType<'a>> TypedList<'a, T> {
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

/// Blanket `DialectNodeType` for `TypedList` — resolves the `NodeId` as a list node.
impl<'a, T> DialectNodeType<'a> for TypedList<'a, T> {
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

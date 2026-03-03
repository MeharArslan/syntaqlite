// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! The two core dialect traits: [`DialectNodeType`] and [`DialectTokenType`].
//!
//! These are symmetric traits that dialect crates implement to plug into the
//! generic typed wrappers in `syntaqlite`:
//!
//! - [`DialectNodeType`] — resolve a typed AST node from the parser arena.
//! - [`DialectTokenType`] — resolve a typed token from a raw token code.

use crate::nodes::NodeId;
use crate::session::ParseResult;

/// A node type that can be resolved from the parser arena by [`NodeId`].
///
/// Implemented by generated view structs (node views, `Node` enum) so that
/// generic containers like `TypedList` can resolve children without
/// dialect-specific code.
///
/// See also the symmetric [`DialectTokenType`] for token enums.
pub trait DialectNodeType<'a>: Sized {
    fn from_arena(reader: ParseResult<'a>, id: NodeId) -> Option<Self>;
}

/// A token type that can be resolved from a raw token integer, and converted
/// back to one.
///
/// Each dialect's token enum must implement this trait to enable generic typed
/// tokenizer and cursor usage.
///
/// See also the symmetric [`DialectNodeType`] for AST node types.
pub trait DialectTokenType: Sized + Clone + Copy + std::fmt::Debug + Into<u32> {
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
    type Node<'a>: DialectNodeType<'a>;
    /// The typed token enum (e.g. `TokenType`).
    type Token: DialectTokenType;
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
    type Node<'a>: DialectNodeType<'a>;
}

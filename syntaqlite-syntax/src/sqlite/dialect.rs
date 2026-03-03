// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! SQLite node family: the syntactic marker type for the SQLite grammar.

use crate::dialect_traits::NodeFamily;

/// Marker type bundling the SQLite AST node and token types for use with
/// [`TypedGrammar<'g, N>`](crate::TypedGrammar).
pub struct SqliteNodeFamily;

impl NodeFamily for SqliteNodeFamily {
    type Node<'a> = super::ast::Stmt<'a>;
    type Token = super::tokens::TokenType;
}

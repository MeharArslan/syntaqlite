// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![warn(unreachable_pub)]

//! SQLite dialect: generated C/Rust artifacts and the dialect handle.
//!
//! Compiles the SQLite Lemon parser, tokenizer, and keyword tables and
//! exports the typed AST nodes, token types, and the [`dialect()`] accessor.

pub mod ast;
pub mod ffi;
pub mod tokens;

pub(crate) mod dialect;

/// Returns the raw (untagged) SQLite dialect handle.
pub use dialect::dialect;

/// Returns the tagged SQLite dialect handle carrying [`SqliteNodeFamily`] type info.
pub use dialect::typed_dialect;

/// Marker type bundling the SQLite AST node and token types for use with
/// [`syntaqlite_parser::TypedDialectEnv<'d, N>`].
pub struct SqliteNodeFamily;

impl syntaqlite_parser::NodeFamily for SqliteNodeFamily {
    type Node<'a> = ast::Stmt<'a>;
    type Token = tokens::TokenType;
}

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! SQLite dialect: generated C/Rust artifacts and the dialect handle.
//!
//! Compiles the SQLite Lemon parser, tokenizer, and keyword tables and
//! exports the typed AST nodes, token types, and the `DIALECT` static.

pub mod ast;
pub mod ffi;
pub mod tokens;

pub(crate) mod dialect;

/// The SQLite dialect handle.
///
/// Use `syntaqlite_parser_sqlite::DIALECT` to get the static dialect handle.
pub use dialect::DIALECT;

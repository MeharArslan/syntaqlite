// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! SQLite dialect: generated C/Rust artifacts and the dialect handle.
//!
//! Compiles the SQLite Lemon parser, tokenizer, and keyword tables and
//! exports the typed AST nodes, token types, and the [`dialect()`] accessor.

pub mod ast;
pub mod ffi;
pub mod tokens;
pub mod wrappers;

pub(crate) mod dialect;

/// Returns the SQLite dialect handle.
pub use dialect::dialect;

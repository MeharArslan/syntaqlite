// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Built-in `SQLite` dialect: AST types, token types, and grammar handle.
//!
//! - **Grammar** — use `grammar::grammar()` to obtain a `SqliteGrammar` handle
//!   and pass it to [`crate::Tokenizer`] or [`crate::Parser`].
//! - **AST** — typed node structs produced by the parser are re-exported from
//!   this module's top level.
//! - **Tokens** — `tokens::TokenType` is the typed token enum produced by the tokenizer.
pub use ast::*;

pub(crate) mod ast;
pub(crate) mod cflags;
pub(crate) mod ffi;
pub(crate) mod grammar;
pub(crate) mod tokens;

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::similar_names))]

//! TODO(claude): write documentation.

// ==== Public API ====

pub use grammar::{RawGrammar, TypedGrammar};
pub use tokenizer::{
    RawToken, RawTokenCursor, RawTokenizer, TypedToken, TypedTokenCursor, TypedTokenizer,
};
#[cfg(feature = "sqlite")]
pub use tokenizer::{Token, TokenCursor, Tokenizer};
pub use version::SqliteVersion;

// ==== Internal modules ====

mod ast;
mod ast_traits;
mod cflags;
mod grammar;
mod incremental;
mod parser;
mod tokenizer;
mod version;

#[cfg(feature = "sqlite")]
mod sqlite;

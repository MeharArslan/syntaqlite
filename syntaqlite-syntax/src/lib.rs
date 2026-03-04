// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::similar_names))]

//! TODO(claude): write documentation.

// ==== Public API ====

pub use grammar::{RawGrammar, TypedGrammar};
pub use tokenizer::{RawToken, RawTokenizer, TokenCursor, TypedToken, TypedTokenCursor, TypedTokenizer};
#[cfg(feature = "sqlite")]
pub use tokenizer::Tokenizer;

// ==== Internal modules ====

mod ast;
mod ast_traits;
mod cflags;
mod grammar;
mod incremental;
mod parser;
mod tokenizer;

#[cfg(feature = "sqlite")]
mod sqlite;

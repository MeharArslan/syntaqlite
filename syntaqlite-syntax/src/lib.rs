// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! TODO(claude): write documentation.

// ==== Public API ====

pub use grammar::Grammar;

// ==== Internal modules ====

mod ast_traits;
mod cflags;
mod dialect_traits;
mod grammar;
mod incremental;
mod node;
mod parser;
mod raw_session;
mod raw_tokenizer;
mod session;

#[cfg(feature = "sqlite")]
mod sqlite;

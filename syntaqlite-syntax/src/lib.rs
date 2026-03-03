// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! TODO(claude): write documentation.

// ==== Public API ====

pub use grammar::Grammar;

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

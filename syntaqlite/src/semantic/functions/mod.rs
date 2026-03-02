// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Function catalog for semantic analysis.
//!
//! Provides [`FunctionCatalog`] — a resolved catalog of known functions for
//! a given dialect + configuration. Supports name/arity checking, lookup,
//! iteration, and completions. Merges SQLite built-in functions, dialect
//! extensions, and session/document-defined functions.

mod catalog;
mod types;

pub use catalog::FunctionCatalog;
pub use types::{FunctionCheckResult, FunctionLookup, SessionFunction};

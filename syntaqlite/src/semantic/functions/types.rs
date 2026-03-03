// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use syntaqlite_parser::FunctionCategory;

/// A user-defined or database function (from DDL parsing, JSON config, or runtime).
///
/// Symmetric with [`RelationDef`](crate::semantic::relations::RelationDef) —
/// both represent externally-defined schema objects.
#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    /// `None` = variadic (any number of arguments).
    pub args: Option<usize>,
}

/// Backward-compatible alias.
pub type SessionFunction = FunctionDef;

/// Result of checking a function call against the catalog.
pub enum FunctionCheckResult {
    /// Function exists and the given arity is accepted.
    Ok,
    /// No function with this name exists.
    Unknown,
    /// Function exists but the arity doesn't match.
    WrongArity {
        /// The arities that would be accepted.
        expected: Vec<usize>,
    },
}

/// Information about a resolved function, returned by [`super::FunctionCatalog::lookup()`].
pub struct FunctionLookup<'a> {
    pub name: &'a str,
    pub category: FunctionCategory,
    /// All accepted fixed arities. Empty + `is_variadic` means any arity.
    pub fixed_arities: Vec<usize>,
    /// Whether this function accepts arbitrary argument counts.
    pub is_variadic: bool,
}

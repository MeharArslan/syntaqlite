// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! SQLite grammar handle: the `grammar()` and `typed_grammar()` accessors.

use std::sync::LazyLock;

use crate::cflags;
use crate::grammar::Grammar;
use crate::typed_grammar::TypedGrammar;

use super::dialect::SqliteNodeFamily;

unsafe extern "C" {
    fn syntaqlite_sqlite_grammar() -> cflags::Grammar;
}

static GRAMMAR: LazyLock<Grammar<'static>> =
    LazyLock::new(|| unsafe { Grammar::from_ffi(syntaqlite_sqlite_grammar()) });

/// Returns the raw (untagged) SQLite grammar handle.
pub fn grammar() -> Grammar<'static> {
    *GRAMMAR
}

/// Returns the SQLite grammar handle tagged with [`SqliteNodeFamily`].
pub fn typed_grammar() -> TypedGrammar<'static, SqliteNodeFamily> {
    TypedGrammar::new(*GRAMMAR)
}

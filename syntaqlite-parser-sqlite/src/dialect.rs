// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! SQLite dialect handle: the `dialect()` accessor and the C FFI declaration.

use std::sync::LazyLock;

use syntaqlite_parser::{Dialect, DialectEnv};

unsafe extern "C" {
    fn syntaqlite_sqlite_dialect() -> *const core::ffi::c_void;
}

static DIALECT: LazyLock<Dialect<'static>> =
    LazyLock::new(|| unsafe { Dialect::from_raw(syntaqlite_sqlite_dialect()) });

/// Returns the raw (untagged) SQLite dialect handle.
pub fn dialect() -> DialectEnv<'static> {
    DialectEnv::new(*DIALECT)
}

/// Returns the SQLite dialect handle tagged with [`SqliteNodeFamily`](crate::SqliteNodeFamily).
pub fn typed_dialect() -> syntaqlite_parser::TypedDialectEnv<'static, crate::SqliteNodeFamily> {
    syntaqlite_parser::TypedDialectEnv::new(*DIALECT)
}

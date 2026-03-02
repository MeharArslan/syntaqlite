// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! SQLite dialect handle: the `dialect()` accessor and the C FFI declaration.

use std::sync::LazyLock;

use syntaqlite_parser::Dialect;

unsafe extern "C" {
    fn syntaqlite_sqlite_dialect() -> *const syntaqlite_parser::FfiDialect;
}

static DIALECT: LazyLock<Dialect<'static>> =
    LazyLock::new(|| unsafe { Dialect::from_raw(syntaqlite_sqlite_dialect()) });

/// Returns the SQLite dialect handle.
pub fn dialect() -> Dialect<'static> {
    *DIALECT
}

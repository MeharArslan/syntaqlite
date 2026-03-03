// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! SQLite dialect handle: the `dialect()` accessor and the C FFI declaration.

use std::sync::LazyLock;

use syntaqlite_parser::DialectEnv;

unsafe extern "C" {
    fn syntaqlite_sqlite_dialect() -> *const syntaqlite_parser::FfiDialect;
}

static DIALECT: LazyLock<DialectEnv<'static>> =
    LazyLock::new(|| unsafe { DialectEnv::from_raw(syntaqlite_sqlite_dialect()) });

/// Returns the raw (untagged) SQLite dialect handle.
pub fn dialect() -> DialectEnv<'static> {
    *DIALECT
}

/// Returns the SQLite dialect handle tagged with [`SqliteNodeFamily`](crate::SqliteNodeFamily).
pub fn tagged_dialect() -> syntaqlite_parser::Dialect<'static, crate::SqliteNodeFamily> {
    syntaqlite_parser::Dialect::from_raw_dialect(*DIALECT)
}

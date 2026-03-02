// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! SQLite dialect handle: the `dialect()` accessor and the C FFI declaration.

use std::sync::LazyLock;

use syntaqlite_parser::dialect::ffi as dialect_ffi;

unsafe extern "C" {
    fn syntaqlite_sqlite_dialect() -> *const dialect_ffi::Dialect;
}

static DIALECT: LazyLock<syntaqlite_parser::dialect::Dialect<'static>> = LazyLock::new(|| unsafe {
    syntaqlite_parser::dialect::Dialect::from_raw(syntaqlite_sqlite_dialect())
});

/// Returns the SQLite dialect handle.
pub fn dialect() -> syntaqlite_parser::dialect::Dialect<'static> {
    *DIALECT
}

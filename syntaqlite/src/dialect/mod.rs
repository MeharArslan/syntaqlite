// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Dialect handle and token classification.

pub(crate) mod catalog;
pub(crate) mod dialect;
pub(crate) mod schema;

pub(crate) use dialect::Dialect;

/// Return the built-in `SQLite` dialect handle.
#[cfg(feature = "sqlite")]
pub(crate) fn sqlite() -> Dialect {
    crate::sqlite::dialect::dialect()
}

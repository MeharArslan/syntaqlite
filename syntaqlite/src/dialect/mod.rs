// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Dialect handle and token classification.

pub(crate) mod catalog;
pub(crate) mod handle;
pub(crate) mod schema;

pub use handle::Dialect;
pub use syntaqlite_syntax::any::TokenCategory;

/// Return the built-in SQLite dialect handle.
#[cfg(feature = "sqlite")]
pub fn sqlite() -> Dialect<'static> {
    crate::sqlite::dialect::dialect()
}

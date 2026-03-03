// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! SQLite dialect — semantic handle, function catalog, and cflag helpers.

pub(crate) mod cflag_helpers;
pub(crate) mod dialect;
pub(crate) mod fmt_statics;
pub(crate) mod functions_catalog;

pub use cflag_helpers::{
    available_functions, cflag_names, cflag_table, parse_cflag_name, parse_sqlite_version,
};
pub use dialect::{dialect, typed_dialect};

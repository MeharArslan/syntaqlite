// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Cflags that add or remove virtual table modules.
//!
//! These are separate from SQL functions (in `functions.rs`) because virtual
//! tables are registered via `sqlite3_create_module()` and show up in
//! `PRAGMA module_list`, not `PRAGMA function_list`.
//!
//! Note: some extensions (FTS3, FTS5, Geopoly) add *both* virtual tables and
//! SQL functions. Those appear in `functions.rs` for the SQL function side and
//! here for the virtual table side.

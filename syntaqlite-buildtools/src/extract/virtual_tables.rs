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

/// Cflags that affect virtual table availability.
///
/// Each entry is (`flag_name`, polarity, `compile_defines`).
/// - OMIT flags: default = OFF. Turning ON removes virtual tables.
/// - ENABLE flags: default = OFF. Turning ON adds virtual tables.
pub const VIRTUAL_TABLE_CFLAGS: &[(&str, &str, &[&str])] = &[
    // OMIT flags.
    //
    // Disables the entire virtual table mechanism, including all modules.
    (
        "SQLITE_OMIT_VIRTUALTABLE",
        "omit",
        &["-DSQLITE_OMIT_VIRTUALTABLE"],
    ),
    // ENABLE flags.
    //
    // FTS3/4 full-text search virtual tables (fts3, fts4).
    ("SQLITE_ENABLE_FTS3", "enable", &["-DSQLITE_ENABLE_FTS3"]),
    // FTS3/4 — listed separately in docs; FTS4 is a superset of FTS3.
    ("SQLITE_ENABLE_FTS4", "enable", &["-DSQLITE_ENABLE_FTS4"]),
    // FTS5 full-text search virtual table (fts5).
    ("SQLITE_ENABLE_FTS5", "enable", &["-DSQLITE_ENABLE_FTS5"]),
    // R*Tree spatial index virtual table (rtree, rtree_i32).
    ("SQLITE_ENABLE_RTREE", "enable", &["-DSQLITE_ENABLE_RTREE"]),
    // Geopoly extension virtual table (geopoly). Requires RTREE.
    (
        "SQLITE_ENABLE_GEOPOLY",
        "enable",
        &["-DSQLITE_ENABLE_GEOPOLY", "-DSQLITE_ENABLE_RTREE"],
    ),
    // DBPAGE virtual table — read/write access to database pages.
    (
        "SQLITE_ENABLE_DBPAGE_VTAB",
        "enable",
        &["-DSQLITE_ENABLE_DBPAGE_VTAB"],
    ),
    // dbstat virtual table — storage statistics per table/index.
    (
        "SQLITE_ENABLE_DBSTAT_VTAB",
        "enable",
        &["-DSQLITE_ENABLE_DBSTAT_VTAB"],
    ),
    // bytecode and tables_used virtual tables.
    (
        "SQLITE_ENABLE_BYTECODE_VTAB",
        "enable",
        &["-DSQLITE_ENABLE_BYTECODE_VTAB"],
    ),
    // SQLITE_STMT virtual table — introspection of prepared statements.
    (
        "SQLITE_ENABLE_STMTVTAB",
        "enable",
        &["-DSQLITE_ENABLE_STMTVTAB"],
    ),
    // carray table-valued function (virtual table, not in PRAGMA function_list).
    (
        "SQLITE_ENABLE_CARRAY",
        "enable",
        &["-DSQLITE_ENABLE_CARRAY"],
    ),
];

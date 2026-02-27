// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Stage 1: Extract C fragments from raw SQLite source.
//!
//! This module is only compiled when the `sqlite-extract` feature is enabled.
//! It reads raw SQLite source files (tokenize.c, global.c, sqliteInt.h,
//! mkkeywordhash.c) and produces the committed fragment files in
//! `data/sqlite_fragments/`.

/// SQLite's public domain blessing header, prepended to all extracted fragments
/// to preserve proper attribution.
pub const SQLITE_BLESSING: &str = "\
/*
** The author disclaims copyright to this source code.  In place of
** a legal notice, here is a blessing:
**
**    May you do good and not evil.
**    May you find forgiveness for yourself and forgive others.
**    May you share freely, never taking more than you give.
*/
";

pub mod amalgamation_probe;
pub mod base_files;
pub mod functions;
pub mod keywords_and_parser;
pub mod mkkeywordhash;
pub mod tokenizer;
pub mod virtual_tables;

// ---------------------------------------------------------------------------
// SYNQ cflag table — the union of all cflag lists
// ---------------------------------------------------------------------------

/// SYNQ cflag index table, mirroring `sqlite_cflags.h`.
///
/// This is the authoritative Rust-side table. It is the union of all cflags
/// across [`keywords_and_parser::PARSER_CFLAGS`], [`functions::FUNCTION_CFLAGS`],
/// and [`virtual_tables::VIRTUAL_TABLE_CFLAGS`].
///
/// Each entry is (sqlite_flag_name, synq_index_constant_name, index).
/// Sorted alphabetically within OMIT and ENABLE groups, indices assigned sequentially.
/// Flags may appear in multiple per-module lists (noted in comments).
pub const SYNQ_CFLAG_TABLE: &[(&str, &str, u32)] = &[
    // ── OMIT flags (0–24) ───────────────────────────────────────────────
    ("SQLITE_OMIT_ALTERTABLE", "SYNQ_CFLAG_OMIT_ALTERTABLE", 0), // parser
    ("SQLITE_OMIT_ANALYZE", "SYNQ_CFLAG_OMIT_ANALYZE", 1),       // parser
    ("SQLITE_OMIT_ATTACH", "SYNQ_CFLAG_OMIT_ATTACH", 2),         // parser
    (
        "SQLITE_OMIT_AUTOINCREMENT",
        "SYNQ_CFLAG_OMIT_AUTOINCREMENT",
        3,
    ), // parser
    ("SQLITE_OMIT_CAST", "SYNQ_CFLAG_OMIT_CAST", 4),             // parser
    (
        "SQLITE_OMIT_COMPILEOPTION_DIAGS",
        "SYNQ_CFLAG_OMIT_COMPILEOPTION_DIAGS",
        5,
    ), // functions
    (
        "SQLITE_OMIT_COMPOUND_SELECT",
        "SYNQ_CFLAG_OMIT_COMPOUND_SELECT",
        6,
    ), // parser
    ("SQLITE_OMIT_CTE", "SYNQ_CFLAG_OMIT_CTE", 7),               // parser
    (
        "SQLITE_OMIT_DATETIME_FUNCS",
        "SYNQ_CFLAG_OMIT_DATETIME_FUNCS",
        8,
    ), // functions
    ("SQLITE_OMIT_EXPLAIN", "SYNQ_CFLAG_OMIT_EXPLAIN", 9),       // parser
    (
        "SQLITE_OMIT_FLOATING_POINT",
        "SYNQ_CFLAG_OMIT_FLOATING_POINT",
        10,
    ), // functions (compile-probing excluded)
    ("SQLITE_OMIT_FOREIGN_KEY", "SYNQ_CFLAG_OMIT_FOREIGN_KEY", 11), // parser
    (
        "SQLITE_OMIT_GENERATED_COLUMNS",
        "SYNQ_CFLAG_OMIT_GENERATED_COLUMNS",
        12,
    ), // parser
    ("SQLITE_OMIT_JSON", "SYNQ_CFLAG_OMIT_JSON", 13),            // functions
    (
        "SQLITE_OMIT_LOAD_EXTENSION",
        "SYNQ_CFLAG_OMIT_LOAD_EXTENSION",
        14,
    ), // functions
    ("SQLITE_OMIT_PRAGMA", "SYNQ_CFLAG_OMIT_PRAGMA", 15),        // parser
    ("SQLITE_OMIT_REINDEX", "SYNQ_CFLAG_OMIT_REINDEX", 16),      // parser
    ("SQLITE_OMIT_RETURNING", "SYNQ_CFLAG_OMIT_RETURNING", 17),  // parser
    ("SQLITE_OMIT_SUBQUERY", "SYNQ_CFLAG_OMIT_SUBQUERY", 18),    // parser
    ("SQLITE_OMIT_TEMPDB", "SYNQ_CFLAG_OMIT_TEMPDB", 19),        // parser
    ("SQLITE_OMIT_TRIGGER", "SYNQ_CFLAG_OMIT_TRIGGER", 20),      // parser
    ("SQLITE_OMIT_VACUUM", "SYNQ_CFLAG_OMIT_VACUUM", 21),        // parser
    ("SQLITE_OMIT_VIEW", "SYNQ_CFLAG_OMIT_VIEW", 22),            // parser
    (
        "SQLITE_OMIT_VIRTUALTABLE",
        "SYNQ_CFLAG_OMIT_VIRTUALTABLE",
        23,
    ), // parser, vtable
    ("SQLITE_OMIT_WINDOWFUNC", "SYNQ_CFLAG_OMIT_WINDOWFUNC", 24), // parser, functions
    // ── ENABLE / misc flags (25–41) ──────────────────────────────────────
    (
        "SQLITE_ENABLE_BYTECODE_VTAB",
        "SYNQ_CFLAG_ENABLE_BYTECODE_VTAB",
        25,
    ), // vtable
    ("SQLITE_ENABLE_CARRAY", "SYNQ_CFLAG_ENABLE_CARRAY", 26), // vtable
    (
        "SQLITE_ENABLE_DBPAGE_VTAB",
        "SYNQ_CFLAG_ENABLE_DBPAGE_VTAB",
        27,
    ), // vtable
    (
        "SQLITE_ENABLE_DBSTAT_VTAB",
        "SYNQ_CFLAG_ENABLE_DBSTAT_VTAB",
        28,
    ), // vtable
    ("SQLITE_ENABLE_FTS3", "SYNQ_CFLAG_ENABLE_FTS3", 29),     // functions, vtable
    ("SQLITE_ENABLE_FTS4", "SYNQ_CFLAG_ENABLE_FTS4", 30),     // vtable
    ("SQLITE_ENABLE_FTS5", "SYNQ_CFLAG_ENABLE_FTS5", 31),     // functions, vtable
    ("SQLITE_ENABLE_GEOPOLY", "SYNQ_CFLAG_ENABLE_GEOPOLY", 32), // functions, vtable
    ("SQLITE_ENABLE_JSON1", "SYNQ_CFLAG_ENABLE_JSON1", 33),   // functions
    (
        "SQLITE_ENABLE_MATH_FUNCTIONS",
        "SYNQ_CFLAG_ENABLE_MATH_FUNCTIONS",
        34,
    ), // functions
    (
        "SQLITE_ENABLE_OFFSET_SQL_FUNC",
        "SYNQ_CFLAG_ENABLE_OFFSET_SQL_FUNC",
        35,
    ), // functions
    (
        "SQLITE_ENABLE_ORDERED_SET_AGGREGATES",
        "SYNQ_CFLAG_ENABLE_ORDERED_SET_AGGREGATES",
        36,
    ), // parser
    (
        "SQLITE_ENABLE_PERCENTILE",
        "SYNQ_CFLAG_ENABLE_PERCENTILE",
        37,
    ), // functions
    ("SQLITE_ENABLE_RTREE", "SYNQ_CFLAG_ENABLE_RTREE", 38),   // vtable
    ("SQLITE_ENABLE_STMTVTAB", "SYNQ_CFLAG_ENABLE_STMTVTAB", 39), // vtable
    (
        "SQLITE_ENABLE_UPDATE_DELETE_LIMIT",
        "SYNQ_CFLAG_ENABLE_UPDATE_DELETE_LIMIT",
        40,
    ), // parser
    ("SQLITE_SOUNDEX", "SYNQ_CFLAG_SOUNDEX", 41),             // functions
];

/// Look up the SYNQ cflag index for a `SQLITE_OMIT_*` or `SQLITE_ENABLE_*` flag.
pub fn synq_cflag_for_sqlite_flag(sqlite_flag: &str) -> Option<u32> {
    SYNQ_CFLAG_TABLE
        .iter()
        .find(|(name, _, _)| *name == sqlite_flag)
        .map(|(_, _, idx)| *idx)
}

#[cfg(test)]
mod tests {
    /// Parse sqlite_cflags.h and verify every entry in SYNQ_CFLAG_TABLE matches.
    #[test]
    fn synq_cflag_table_matches_header() {
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../syntaqlite-runtime/include/syntaqlite/sqlite_cflags.h"
        ));

        // Parse "#define SYNQ_CFLAG_FOO  N" lines from the header.
        let mut header_defines: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();
        for line in header.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("#define SYNQ_CFLAG_") {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = format!("SYNQ_CFLAG_{}", parts[0]);
                    if let Ok(val) = parts[1].parse::<u32>() {
                        header_defines.insert(name, val);
                    }
                }
            }
        }

        for (_, synq_name, index) in super::SYNQ_CFLAG_TABLE {
            let header_val = header_defines.get(*synq_name);
            assert_eq!(
                header_val,
                Some(index),
                "SYNQ_CFLAG_TABLE entry {synq_name}={index} does not match sqlite_cflags.h (got {:?})",
                header_val
            );
        }

        let table_names: std::collections::HashSet<&str> =
            super::SYNQ_CFLAG_TABLE.iter().map(|(_, n, _)| *n).collect();
        for (name, val) in &header_defines {
            if name == "SYNQ_CFLAG_COUNT" {
                continue;
            }
            assert!(
                table_names.contains(name.as_str()),
                "sqlite_cflags.h defines {name}={val} but it is missing from SYNQ_CFLAG_TABLE"
            );
        }
    }
}

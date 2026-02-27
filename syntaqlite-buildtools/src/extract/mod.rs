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

/// SYNQ cflag constants, mirroring `sqlite_cflags.h`.
///
/// This is the authoritative Rust-side table. It is the union of all cflags
/// across [`keywords_and_parser::PARSER_CFLAGS`], [`functions::FUNCTION_CFLAGS`],
/// and [`virtual_tables::VIRTUAL_TABLE_CFLAGS`]. Each flag occupies a single bit.
///
/// Sorted alphabetically within OMIT and ENABLE groups, bits assigned sequentially.
/// Flags may appear in multiple per-module lists (noted in comments).
pub const SYNQ_CFLAG_TABLE: &[(&str, u64)] = &[
    // ── OMIT flags (bits 0–24) ──────────────────────────────────────────
    ("SYNQ_SQLITE_OMIT_ALTERTABLE",                0x0000_0000_0000_0001), // parser
    ("SYNQ_SQLITE_OMIT_ANALYZE",                   0x0000_0000_0000_0002), // parser
    ("SYNQ_SQLITE_OMIT_ATTACH",                    0x0000_0000_0000_0004), // parser
    ("SYNQ_SQLITE_OMIT_AUTOINCREMENT",             0x0000_0000_0000_0008), // parser
    ("SYNQ_SQLITE_OMIT_CAST",                      0x0000_0000_0000_0010), // parser
    ("SYNQ_SQLITE_OMIT_COMPILEOPTION_DIAGS",       0x0000_0000_0000_0020), // functions
    ("SYNQ_SQLITE_OMIT_COMPOUND_SELECT",           0x0000_0000_0000_0040), // parser
    ("SYNQ_SQLITE_OMIT_CTE",                       0x0000_0000_0000_0080), // parser
    ("SYNQ_SQLITE_OMIT_DATETIME_FUNCS",            0x0000_0000_0000_0100), // functions
    ("SYNQ_SQLITE_OMIT_EXPLAIN",                   0x0000_0000_0000_0200), // parser
    ("SYNQ_SQLITE_OMIT_FLOATING_POINT",            0x0000_0000_0000_0400), // functions (compile-probing excluded)
    ("SYNQ_SQLITE_OMIT_FOREIGN_KEY",               0x0000_0000_0000_0800), // parser
    ("SYNQ_SQLITE_OMIT_GENERATED_COLUMNS",         0x0000_0000_0000_1000), // parser
    ("SYNQ_SQLITE_OMIT_JSON",                      0x0000_0000_0000_2000), // functions
    ("SYNQ_SQLITE_OMIT_LOAD_EXTENSION",            0x0000_0000_0000_4000), // functions
    ("SYNQ_SQLITE_OMIT_PRAGMA",                    0x0000_0000_0000_8000), // parser
    ("SYNQ_SQLITE_OMIT_REINDEX",                   0x0000_0000_0001_0000), // parser
    ("SYNQ_SQLITE_OMIT_RETURNING",                 0x0000_0000_0002_0000), // parser
    ("SYNQ_SQLITE_OMIT_SUBQUERY",                  0x0000_0000_0004_0000), // parser
    ("SYNQ_SQLITE_OMIT_TEMPDB",                    0x0000_0000_0008_0000), // parser
    ("SYNQ_SQLITE_OMIT_TRIGGER",                   0x0000_0000_0010_0000), // parser
    ("SYNQ_SQLITE_OMIT_VACUUM",                    0x0000_0000_0020_0000), // parser
    ("SYNQ_SQLITE_OMIT_VIEW",                      0x0000_0000_0040_0000), // parser
    ("SYNQ_SQLITE_OMIT_VIRTUALTABLE",              0x0000_0000_0080_0000), // parser, vtable
    ("SYNQ_SQLITE_OMIT_WINDOWFUNC",                0x0000_0000_0100_0000), // parser, functions
    // ── ENABLE / misc flags (bits 25–41) ─────────────────────────────────
    ("SYNQ_SQLITE_ENABLE_BYTECODE_VTAB",           0x0000_0000_0200_0000), // vtable
    ("SYNQ_SQLITE_ENABLE_CARRAY",                  0x0000_0000_0400_0000), // vtable
    ("SYNQ_SQLITE_ENABLE_DBPAGE_VTAB",             0x0000_0000_0800_0000), // vtable
    ("SYNQ_SQLITE_ENABLE_DBSTAT_VTAB",             0x0000_0000_1000_0000), // vtable
    ("SYNQ_SQLITE_ENABLE_FTS3",                    0x0000_0000_2000_0000), // functions, vtable
    ("SYNQ_SQLITE_ENABLE_FTS4",                    0x0000_0000_4000_0000), // vtable
    ("SYNQ_SQLITE_ENABLE_FTS5",                    0x0000_0000_8000_0000), // functions, vtable
    ("SYNQ_SQLITE_ENABLE_GEOPOLY",                 0x0000_0001_0000_0000), // functions, vtable
    ("SYNQ_SQLITE_ENABLE_JSON1",                   0x0000_0002_0000_0000), // functions
    ("SYNQ_SQLITE_ENABLE_MATH_FUNCTIONS",          0x0000_0004_0000_0000), // functions
    ("SYNQ_SQLITE_ENABLE_OFFSET_SQL_FUNC",         0x0000_0008_0000_0000), // functions
    ("SYNQ_SQLITE_ENABLE_ORDERED_SET_AGGREGATES",  0x0000_0010_0000_0000), // parser
    ("SYNQ_SQLITE_ENABLE_PERCENTILE",              0x0000_0020_0000_0000), // functions
    ("SYNQ_SQLITE_ENABLE_RTREE",                   0x0000_0040_0000_0000), // vtable
    ("SYNQ_SQLITE_ENABLE_STMTVTAB",               0x0000_0080_0000_0000), // vtable
    ("SYNQ_SQLITE_ENABLE_UPDATE_DELETE_LIMIT",     0x0000_0100_0000_0000), // parser
    ("SYNQ_SQLITE_SOUNDEX",                        0x0000_0200_0000_0000), // functions
];

/// Look up the SYNQ cflag bit value for a `SQLITE_OMIT_*` or `SQLITE_ENABLE_*` flag.
pub fn synq_cflag_for_sqlite_flag(sqlite_flag: &str) -> Option<u64> {
    let synq_name = format!("SYNQ_{sqlite_flag}");
    SYNQ_CFLAG_TABLE
        .iter()
        .find(|(name, _)| *name == synq_name)
        .map(|(_, val)| *val)
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

        let mut header_defines: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();
        for line in header.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("#define SYNQ_SQLITE_") {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = format!("SYNQ_SQLITE_{}", parts[0]);
                    // Value is like ((uint64_t)0x...) — extract the hex.
                    let val_str = parts[1..].join("");
                    if let Some(hex_start) = val_str.find("0x") {
                        let hex = &val_str[hex_start + 2..];
                        let hex = hex.trim_end_matches(|c: char| !c.is_ascii_hexdigit());
                        if let Ok(val) = u64::from_str_radix(hex, 16) {
                            header_defines.insert(name, val);
                        }
                    }
                }
            }
        }

        for (name, value) in super::SYNQ_CFLAG_TABLE {
            let header_val = header_defines.get(*name);
            assert_eq!(
                header_val,
                Some(value),
                "SYNQ_CFLAG_TABLE entry {name}={value:#018x} does not match sqlite_cflags.h (got {:?})",
                header_val
            );
        }

        let table_names: std::collections::HashSet<&str> =
            super::SYNQ_CFLAG_TABLE.iter().map(|(n, _)| *n).collect();
        for (name, val) in &header_defines {
            assert!(
                table_names.contains(name.as_str()),
                "sqlite_cflags.h defines {name}={val:#018x} but it is missing from SYNQ_CFLAG_TABLE"
            );
        }
    }
}

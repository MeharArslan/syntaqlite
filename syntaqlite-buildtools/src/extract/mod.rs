// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Stage 1: Extract C fragments from raw `SQLite` source.
//!
//! This module is only compiled when the `sqlite-extract` feature is enabled.
//! It reads raw `SQLite` source files (tokenize.c, global.c, sqliteInt.h,
//! mkkeywordhash.c) and produces the committed fragment files in
//! `data/sqlite_fragments/`.

/// `SQLite`'s public domain blessing header, prepended to all extracted fragments
/// to preserve proper attribution.
pub(crate) const SQLITE_BLESSING: &str = "\
/*
** The author disclaims copyright to this source code.  In place of
** a legal notice, here is a blessing:
**
**    May you do good and not evil.
**    May you find forgiveness for yourself and forgive others.
**    May you share freely, never taking more than you give.
*/
";

pub(crate) mod amalgamation_probe;
pub(crate) mod base_files;
pub(crate) mod functions;
pub(crate) mod keywords_and_parser;
pub(crate) mod mkkeywordhash;
pub(crate) mod tokenizer;
pub(crate) mod virtual_tables;

// ---------------------------------------------------------------------------
// SYNQ cflag table — the union of all cflag lists
// ---------------------------------------------------------------------------

/// SYNQ cflag index table, mirroring `cflags.h`.
///
/// This is the authoritative Rust-side table. It is the union of all cflags
/// across [`keywords_and_parser::PARSER_CFLAGS`], `functions::FUNCTION_CFLAGS`,
/// and [`virtual_tables::VIRTUAL_TABLE_CFLAGS`].
///
/// Each entry is (`sqlite_flag_name`, `synq_index_constant_name`, index, categories).
/// Sorted alphabetically within OMIT and ENABLE groups, indices assigned sequentially.
///
/// A flag may belong to multiple categories when it spans multiple concerns:
/// - `"parser"`:    affects keyword recognition or SQL syntax
/// - `"functions"`: affects built-in function availability
/// - `"vtable"`:    affects virtual table modules
/// - `"extensions"`: enables optional extension modules (FTS, `RTree`, etc.)
pub(crate) const SYNQ_CFLAG_TABLE: &[(&str, &str, u32, &[&str])] = &[
    // ── OMIT flags (0–24) ───────────────────────────────────────────────
    (
        "SQLITE_OMIT_ALTERTABLE",
        "SYNQ_CFLAG_IDX_OMIT_ALTERTABLE",
        0,
        &["parser"],
    ),
    (
        "SQLITE_OMIT_ANALYZE",
        "SYNQ_CFLAG_IDX_OMIT_ANALYZE",
        1,
        &["parser"],
    ),
    (
        "SQLITE_OMIT_ATTACH",
        "SYNQ_CFLAG_IDX_OMIT_ATTACH",
        2,
        &["parser"],
    ),
    (
        "SQLITE_OMIT_AUTOINCREMENT",
        "SYNQ_CFLAG_IDX_OMIT_AUTOINCREMENT",
        3,
        &["parser"],
    ),
    (
        "SQLITE_OMIT_CAST",
        "SYNQ_CFLAG_IDX_OMIT_CAST",
        4,
        &["parser"],
    ),
    (
        "SQLITE_OMIT_COMPILEOPTION_DIAGS",
        "SYNQ_CFLAG_IDX_OMIT_COMPILEOPTION_DIAGS",
        5,
        &["functions"],
    ),
    (
        "SQLITE_OMIT_COMPOUND_SELECT",
        "SYNQ_CFLAG_IDX_OMIT_COMPOUND_SELECT",
        6,
        &["parser"],
    ),
    ("SQLITE_OMIT_CTE", "SYNQ_CFLAG_IDX_OMIT_CTE", 7, &["parser"]),
    (
        "SQLITE_OMIT_DATETIME_FUNCS",
        "SYNQ_CFLAG_IDX_OMIT_DATETIME_FUNCS",
        8,
        &["functions"],
    ),
    (
        "SQLITE_OMIT_EXPLAIN",
        "SYNQ_CFLAG_IDX_OMIT_EXPLAIN",
        9,
        &["parser"],
    ),
    (
        "SQLITE_OMIT_FLOATING_POINT",
        "SYNQ_CFLAG_IDX_OMIT_FLOATING_POINT",
        10,
        &["functions"],
    ),
    (
        "SQLITE_OMIT_FOREIGN_KEY",
        "SYNQ_CFLAG_IDX_OMIT_FOREIGN_KEY",
        11,
        &["parser"],
    ),
    (
        "SQLITE_OMIT_GENERATED_COLUMNS",
        "SYNQ_CFLAG_IDX_OMIT_GENERATED_COLUMNS",
        12,
        &["parser"],
    ),
    (
        "SQLITE_OMIT_JSON",
        "SYNQ_CFLAG_IDX_OMIT_JSON",
        13,
        &["functions"],
    ),
    (
        "SQLITE_OMIT_LOAD_EXTENSION",
        "SYNQ_CFLAG_IDX_OMIT_LOAD_EXTENSION",
        14,
        &["functions"],
    ),
    (
        "SQLITE_OMIT_PRAGMA",
        "SYNQ_CFLAG_IDX_OMIT_PRAGMA",
        15,
        &["parser"],
    ),
    (
        "SQLITE_OMIT_REINDEX",
        "SYNQ_CFLAG_IDX_OMIT_REINDEX",
        16,
        &["parser"],
    ),
    (
        "SQLITE_OMIT_RETURNING",
        "SYNQ_CFLAG_IDX_OMIT_RETURNING",
        17,
        &["parser"],
    ),
    (
        "SQLITE_OMIT_SUBQUERY",
        "SYNQ_CFLAG_IDX_OMIT_SUBQUERY",
        18,
        &["parser"],
    ),
    (
        "SQLITE_OMIT_TEMPDB",
        "SYNQ_CFLAG_IDX_OMIT_TEMPDB",
        19,
        &["parser"],
    ),
    (
        "SQLITE_OMIT_TRIGGER",
        "SYNQ_CFLAG_IDX_OMIT_TRIGGER",
        20,
        &["parser"],
    ),
    (
        "SQLITE_OMIT_VACUUM",
        "SYNQ_CFLAG_IDX_OMIT_VACUUM",
        21,
        &["parser"],
    ),
    (
        "SQLITE_OMIT_VIEW",
        "SYNQ_CFLAG_IDX_OMIT_VIEW",
        22,
        &["parser"],
    ),
    (
        "SQLITE_OMIT_VIRTUALTABLE",
        "SYNQ_CFLAG_IDX_OMIT_VIRTUALTABLE",
        23,
        &["parser", "vtable"], // adds VIRTUAL keyword (parser) + disables vtable mechanism
    ),
    (
        "SQLITE_OMIT_WINDOWFUNC",
        "SYNQ_CFLAG_IDX_OMIT_WINDOWFUNC",
        24,
        &["parser", "functions"], // removes window keywords (parser) + window functions
    ),
    // ── ENABLE / misc flags (25–41) ──────────────────────────────────────
    (
        "SQLITE_ENABLE_BYTECODE_VTAB",
        "SYNQ_CFLAG_IDX_ENABLE_BYTECODE_VTAB",
        25,
        &["vtable"],
    ),
    (
        "SQLITE_ENABLE_CARRAY",
        "SYNQ_CFLAG_IDX_ENABLE_CARRAY",
        26,
        &["vtable"],
    ),
    (
        "SQLITE_ENABLE_DBPAGE_VTAB",
        "SYNQ_CFLAG_IDX_ENABLE_DBPAGE_VTAB",
        27,
        &["vtable"],
    ),
    (
        "SQLITE_ENABLE_DBSTAT_VTAB",
        "SYNQ_CFLAG_IDX_ENABLE_DBSTAT_VTAB",
        28,
        &["vtable"],
    ),
    (
        "SQLITE_ENABLE_FTS3",
        "SYNQ_CFLAG_IDX_ENABLE_FTS3",
        29,
        &["extensions"],
    ),
    (
        "SQLITE_ENABLE_FTS4",
        "SYNQ_CFLAG_IDX_ENABLE_FTS4",
        30,
        &["extensions"],
    ),
    (
        "SQLITE_ENABLE_FTS5",
        "SYNQ_CFLAG_IDX_ENABLE_FTS5",
        31,
        &["extensions"],
    ),
    (
        "SQLITE_ENABLE_GEOPOLY",
        "SYNQ_CFLAG_IDX_ENABLE_GEOPOLY",
        32,
        &["extensions"],
    ),
    (
        "SQLITE_ENABLE_JSON1",
        "SYNQ_CFLAG_IDX_ENABLE_JSON1",
        33,
        &["functions"],
    ),
    (
        "SQLITE_ENABLE_MATH_FUNCTIONS",
        "SYNQ_CFLAG_IDX_ENABLE_MATH_FUNCTIONS",
        34,
        &["functions"],
    ),
    (
        "SQLITE_ENABLE_OFFSET_SQL_FUNC",
        "SYNQ_CFLAG_IDX_ENABLE_OFFSET_SQL_FUNC",
        35,
        &["functions"],
    ),
    (
        "SQLITE_ENABLE_ORDERED_SET_AGGREGATES",
        "SYNQ_CFLAG_IDX_ENABLE_ORDERED_SET_AGGREGATES",
        36,
        &["parser"], // adds WITHIN keyword for ordered-set aggregate syntax
    ),
    (
        "SQLITE_ENABLE_PERCENTILE",
        "SYNQ_CFLAG_IDX_ENABLE_PERCENTILE",
        37,
        &["functions"],
    ),
    (
        "SQLITE_ENABLE_RTREE",
        "SYNQ_CFLAG_IDX_ENABLE_RTREE",
        38,
        &["extensions"],
    ),
    (
        "SQLITE_ENABLE_STMTVTAB",
        "SYNQ_CFLAG_IDX_ENABLE_STMTVTAB",
        39,
        &["vtable"],
    ),
    (
        "SQLITE_ENABLE_UPDATE_DELETE_LIMIT",
        "SYNQ_CFLAG_IDX_ENABLE_UPDATE_DELETE_LIMIT",
        40,
        &["parser"],
    ),
    (
        "SQLITE_SOUNDEX",
        "SYNQ_CFLAG_IDX_SOUNDEX",
        41,
        &["functions"],
    ),
];

/// Look up the SYNQ cflag index for a `SQLITE_OMIT_*` or `SQLITE_ENABLE_*` flag.
#[allow(dead_code)]
#[must_use]
pub(crate) fn synq_cflag_for_sqlite_flag(sqlite_flag: &str) -> Option<u32> {
    SYNQ_CFLAG_TABLE
        .iter()
        .find(|(name, _, _, _)| *name == sqlite_flag)
        .map(|(_, _, idx, _)| *idx)
}

/// Compute the group-local index of `sqlite_flag` within the `group` category.
///
/// Local indices are 0, 1, 2, … assigned by iterating `SYNQ_CFLAG_TABLE` in order
/// and counting only entries whose categories slice contains `group`.
#[allow(dead_code)]
#[must_use]
pub(crate) fn group_local_index(group: &str, sqlite_flag: &str) -> Option<u32> {
    let mut local = 0u32;
    for &(name, _, _, cats) in SYNQ_CFLAG_TABLE {
        if cats.contains(&group) {
            if name == sqlite_flag {
                return Some(local);
            }
            local += 1;
        }
    }
    None
}

fn cflag_field_name(synq_name: &str) -> String {
    synq_name
        .strip_prefix("SYNQ_CFLAG_IDX_")
        .unwrap_or(synq_name)
        .to_lowercase()
}

fn write_cflag_defines(out: &mut String, entries: &[(&str, &str, u32)], count: u32) {
    use std::fmt::Write as _;
    out.push_str("// ── Cflag index constants ───────────────────────────────────────────────\n");
    out.push_str("//\n");
    out.push_str("// These are group-local indices used in keyword/parser tables.\n");
    out.push('\n');
    let mut last_prefix = "";
    for &(_, synq_name, idx) in entries {
        let prefix = if synq_name.contains("OMIT") {
            "OMIT"
        } else {
            "ENABLE"
        };
        if prefix != last_prefix {
            if !last_prefix.is_empty() {
                out.push('\n');
            }
            writeln!(out, "// {prefix} flags:").expect("write to String");
            last_prefix = prefix;
        }
        writeln!(out, "#define {synq_name} {idx}").expect("write to String");
    }
    out.push('\n');
    writeln!(out, "#define SYNQ_CFLAG_IDX_COUNT {count}").expect("write to String");
    out.push('\n');
}

fn write_cflag_struct(
    out: &mut String,
    entries: &[(&str, &str, u32)],
    bits_total: u32,
    padding: u32,
    byte_count: u32,
) {
    use std::fmt::Write as _;
    out.push_str("// ── Named bitfield struct ───────────────────────────────────────────────\n");
    out.push('\n');
    out.push_str("typedef struct SyntaqliteCflags {\n");
    let mut last_prefix = "";
    for &(_, synq_name, _) in entries {
        let prefix = if synq_name.contains("OMIT") {
            "OMIT"
        } else {
            "ENABLE"
        };
        if prefix != last_prefix {
            writeln!(out, "  // {prefix} flags:").expect("write to String");
            last_prefix = prefix;
        }
        let field = cflag_field_name(synq_name);
        writeln!(out, "  uint8_t {field} : 1;").expect("write to String");
    }
    if padding > 0 {
        writeln!(
            out,
            "  // Padding to {bits_total} bits ({byte_count} bytes):"
        )
        .expect("write to String");
        writeln!(out, "  uint8_t _reserved : {padding};").expect("write to String");
    }
    out.push_str("} SyntaqliteCflags;\n");
    out.push('\n');
    out.push_str("#define SYNQ_CFLAGS_DEFAULT {0}\n");
    out.push('\n');
}

fn write_cflag_pinning(out: &mut String, entries: &[(&str, &str, u32)]) {
    use std::fmt::Write as _;
    out.push_str("// ── Compile-time cflag pinning ──────────────────────────────────────────\n");
    out.push_str("//\n");
    out.push_str("// When SYNTAQLITE_SQLITE_CFLAGS is defined, a static const struct is built\n");
    out.push_str("// from individual SYNTAQLITE_CFLAG_* defines.\n");
    out.push('\n');
    out.push_str("#ifdef SYNTAQLITE_SQLITE_CFLAGS\n");
    out.push_str("static const SyntaqliteCflags synq_pinned_cflags = {\n");
    for &(sqlite_flag, synq_name, _) in entries {
        let field = cflag_field_name(synq_name);
        writeln!(out, "#ifdef SYNTAQLITE_CFLAG_{sqlite_flag}").expect("write to String");
        writeln!(out, "    .{field} = 1,").expect("write to String");
        out.push_str("#endif\n");
    }
    out.push_str("};\n");
    out.push_str("#endif  // SYNTAQLITE_SQLITE_CFLAGS\n");
}

/// Generate the `cflags.h` C header for a given cflag group.
///
/// Only entries whose categories contain `group` are included.
/// Indices are re-assigned 0, 1, 2, … (group-local) in table order.
///
/// # Panics
///
/// Never in practice; panics only if the number of cflags exceeds `u32::MAX`.
#[allow(dead_code)]
#[must_use]
pub(crate) fn generate_cflags_h(group: &str) -> String {
    use std::fmt::Write as _;
    // Collect group entries with sequential local indices.
    let mut local_idx: u32 = 0;
    let entries: Vec<(&str, &str, u32)> = SYNQ_CFLAG_TABLE
        .iter()
        .filter_map(|&(sqlite_flag, synq_name, _, cats)| {
            if cats.contains(&group) {
                let idx = local_idx;
                local_idx += 1;
                Some((sqlite_flag, synq_name, idx))
            } else {
                None
            }
        })
        .collect();

    let count = u32::try_from(entries.len()).expect("cflag count fits u32");
    let bits_total = count.div_ceil(8) * 8;
    let padding = bits_total - count;
    let byte_count = bits_total / 8;

    let mut out = String::new();
    out.push_str("// Copyright 2025 The syntaqlite Authors. All rights reserved.\n");
    out.push_str("// Licensed under the Apache License, Version 2.0.\n");
    out.push_str("// @generated by sqlite-extract — DO NOT EDIT\n");
    out.push_str("//\n");
    writeln!(
        out,
        "// SQLite compile-time flag constants for the \"{group}\" group."
    )
    .expect("write to String");
    out.push_str("// For use with SyntaqliteGrammar.cflags.\n");
    out.push_str("//\n");
    out.push_str("// Indices are group-local (0-based within this group).\n");
    out.push('\n');
    out.push_str("#ifndef SYNTAQLITE_SQLITE_CFLAGS_H\n");
    out.push_str("#define SYNTAQLITE_SQLITE_CFLAGS_H\n");
    out.push('\n');
    out.push_str("#include <stdint.h>\n");
    out.push_str("#include <string.h>\n");
    out.push('\n');
    write_cflag_defines(&mut out, &entries, count);
    write_cflag_struct(&mut out, &entries, bits_total, padding, byte_count);
    out.push_str("// ── Indexed accessor ────────────────────────────────────────────────────\n");
    out.push_str("//\n");
    out.push_str("// For dynamic cflag lookup (keyword tables etc.).\n");
    out.push('\n');
    out.push_str("static inline int synq_has_cflag(const SyntaqliteCflags* c, int idx) {\n");
    out.push_str("  const uint8_t* bytes = (const uint8_t*)c;\n");
    out.push_str("  return (bytes[idx / 8] >> (idx % 8)) & 1;\n");
    out.push_str("}\n");
    out.push('\n');
    out.push_str("static inline void synq_set_cflag(SyntaqliteCflags* c, int idx) {\n");
    out.push_str("  uint8_t* bytes = (uint8_t*)c;\n");
    out.push_str("  bytes[idx / 8] |= (uint8_t)(1u << (idx % 8));\n");
    out.push_str("}\n");
    out.push('\n');
    write_cflag_pinning(&mut out, &entries);
    out.push('\n');
    out.push_str("#endif  // SYNTAQLITE_SQLITE_CFLAGS_H\n");
    out
}

#[cfg(test)]
mod tests {
    /// Verify that `generate_cflags_h("parser")` is self-consistent: only parser-group
    /// entries from `SYNQ_CFLAG_TABLE` appear, with correct group-local indices.
    #[test]
    fn generate_cflags_h_parser_is_consistent() {
        let generated = super::generate_cflags_h("parser");

        // Parse "#define SYNQ_CFLAG_IDX_*  N" lines from the generated header.
        let mut defines: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        for line in generated.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("#define SYNQ_CFLAG_IDX_") {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = format!("SYNQ_CFLAG_IDX_{}", parts[0]);
                    if let Ok(val) = parts[1].parse::<u32>() {
                        defines.insert(name, val);
                    }
                }
            }
        }

        // Every parser-group entry must appear with its group-local index.
        for &(sqlite_flag, synq_name, _, cats) in super::SYNQ_CFLAG_TABLE {
            if !cats.contains(&"parser") {
                // Non-parser entries must NOT appear.
                assert!(
                    !defines.contains_key(synq_name),
                    "non-parser flag {synq_name} should not be in parser cflags.h"
                );
                continue;
            }
            let expected_local = super::group_local_index("parser", sqlite_flag)
                .expect("parser-group flag must have a local index");
            let header_val = defines.get(synq_name);
            assert_eq!(
                header_val,
                Some(&expected_local),
                "{synq_name}: expected local index {expected_local}, got {header_val:?}"
            );
        }

        // Every define in the header (except COUNT) must correspond to a parser-group entry.
        let parser_synq_names: std::collections::HashSet<&str> = super::SYNQ_CFLAG_TABLE
            .iter()
            .filter(|(_, _, _, cats)| cats.contains(&"parser"))
            .map(|(_, n, _, _)| *n)
            .collect();
        for (name, val) in &defines {
            if name == "SYNQ_CFLAG_IDX_COUNT" {
                continue;
            }
            assert!(
                parser_synq_names.contains(name.as_str()),
                "cflags.h defines {name}={val} but it is not a parser-group entry in SYNQ_CFLAG_TABLE"
            );
        }
    }

}

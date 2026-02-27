// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Stage 1 keyword cflag extraction: parse mkkeywordhash.c to produce the
//! keyword → (cflag, polarity) mapping.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::util::mkkeywordhash_parser;

/// SYNQ cflag constants, mirroring `sqlite_cflags.h`.
const SYNQ_CFLAG_TABLE: &[(&str, u32)] = &[
    ("SYNQ_SQLITE_OMIT_EXPLAIN", 0x00000001),
    ("SYNQ_SQLITE_OMIT_TEMPDB", 0x00000002),
    ("SYNQ_SQLITE_OMIT_COMPOUND_SELECT", 0x00000004),
    ("SYNQ_SQLITE_OMIT_WINDOWFUNC", 0x00000008),
    ("SYNQ_SQLITE_OMIT_GENERATED_COLUMNS", 0x00000010),
    ("SYNQ_SQLITE_OMIT_VIEW", 0x00000020),
    ("SYNQ_SQLITE_OMIT_CTE", 0x00000040),
    ("SYNQ_SQLITE_OMIT_SUBQUERY", 0x00000080),
    ("SYNQ_SQLITE_OMIT_CAST", 0x00000100),
    ("SYNQ_SQLITE_OMIT_PRAGMA", 0x00000200),
    ("SYNQ_SQLITE_OMIT_TRIGGER", 0x00000400),
    ("SYNQ_SQLITE_OMIT_ATTACH", 0x00000800),
    ("SYNQ_SQLITE_OMIT_REINDEX", 0x00001000),
    ("SYNQ_SQLITE_OMIT_ANALYZE", 0x00002000),
    ("SYNQ_SQLITE_OMIT_ALTERTABLE", 0x00004000),
    ("SYNQ_SQLITE_OMIT_VIRTUALTABLE", 0x00008000),
    ("SYNQ_SQLITE_OMIT_RETURNING", 0x00010000),
    ("SYNQ_SQLITE_ENABLE_ORDERED_SET_AGGREGATES", 0x00020000),
];

/// Look up the SYNQ cflag value for a `SQLITE_OMIT_*` or `SQLITE_ENABLE_*` flag.
fn synq_cflag_for_sqlite_flag(sqlite_flag: &str) -> Option<u32> {
    let synq_name = format!("SYNQ_{sqlite_flag}");
    SYNQ_CFLAG_TABLE
        .iter()
        .find(|(name, _)| *name == synq_name)
        .map(|(_, val)| *val)
}

/// Extract keyword cflag data from mkkeywordhash.c source.
///
/// Returns a map of keyword name → (cflag_value, polarity).
pub fn extract_keyword_cflags(mkkeywordhash_source: &str) -> Result<HashMap<String, (u32, u8)>, String> {
    let table = mkkeywordhash_parser::parse_keyword_table(mkkeywordhash_source)?;

    // Build mask_name → (omit_flag, polarity) lookup.
    let mask_lookup: HashMap<&str, (&str, u8)> = table
        .masks
        .iter()
        .map(|m| (m.name.as_str(), (m.omit_flag.as_str(), m.polarity)))
        .collect();

    let mut map = HashMap::new();

    for kw in &table.keywords {
        // Skip keywords with ALWAYS mask — always enabled.
        if kw.mask_expr == "ALWAYS" {
            continue;
        }
        // Skip OR'd masks (e.g. "CONFLICT|TRIGGER") — no single SYNQ constant.
        if kw.mask_expr.contains('|') {
            continue;
        }
        // Look up the mask symbol in the defines.
        if let Some(&(omit_flag, polarity)) = mask_lookup.get(kw.mask_expr.as_str()) {
            if let Some(cflag_val) = synq_cflag_for_sqlite_flag(omit_flag) {
                map.insert(kw.name.clone(), (cflag_val, polarity));
            }
        }
    }

    Ok(map)
}

/// Write keyword cflag data to a file.
///
/// Format: tab-separated lines of `KEYWORD_NAME\tcflag_value\tpolarity`.
pub fn write_keyword_cflags(
    cflags: &HashMap<String, (u32, u8)>,
    output_path: &Path,
) -> Result<(), String> {
    let mut lines: Vec<String> = Vec::new();
    lines.push("# Keyword compile-flag data extracted from mkkeywordhash.c".to_string());
    lines.push("# Format: KEYWORD_NAME<tab>cflag_value<tab>polarity".to_string());
    lines.push(
        "# POLARITY: 0 = OMIT (keyword disabled when flag set), 1 = ENABLE (keyword enabled when flag set)"
            .to_string(),
    );

    let mut entries: Vec<_> = cflags.iter().collect();
    entries.sort_by_key(|(name, _)| (*name).clone());

    for (name, (cflag, polarity)) in entries {
        lines.push(format!("{name}\t{cflag}\t{polarity}"));
    }
    lines.push(String::new()); // trailing newline

    fs::write(output_path, lines.join("\n"))
        .map_err(|e| format!("writing {}: {e}", output_path.display()))
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

        let mut header_defines: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();
        for line in header.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("#define SYNQ_SQLITE_") {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = format!("SYNQ_SQLITE_{}", parts[0]);
                    if let Some(hex) = parts[1].strip_prefix("0x") {
                        if let Ok(val) = u32::from_str_radix(hex, 16) {
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
                "SYNQ_CFLAG_TABLE entry {name}={value:#010x} does not match sqlite_cflags.h (got {:?})",
                header_val
            );
        }

        let table_names: std::collections::HashSet<&str> =
            super::SYNQ_CFLAG_TABLE.iter().map(|(n, _)| *n).collect();
        for (name, val) in &header_defines {
            assert!(
                table_names.contains(name.as_str()),
                "sqlite_cflags.h defines {name}={val:#010x} but it is missing from SYNQ_CFLAG_TABLE"
            );
        }
    }
}

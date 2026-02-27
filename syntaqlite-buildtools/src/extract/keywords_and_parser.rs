// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Cflags that affect the parser grammar and keyword table.
//!
//! These flags gate SQL syntax — they add or remove keywords, statements, or
//! clauses from the parser. They must be known at parser-generation time
//! (mkkeywordhash + Lemon grammar).
//!
//! Also contains the keyword cflag extraction logic that reads mkkeywordhash.c
//! to produce the keyword → (cflag, polarity) mapping.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::util::mkkeywordhash_parser;

// ---------------------------------------------------------------------------
// Parser cflag catalog
// ---------------------------------------------------------------------------

/// Cflags that affect parser grammar and/or keyword availability.
///
/// Each entry is (flag_name, polarity, description).
/// - "omit": default = OFF. Turning ON removes syntax.
/// - "enable": default = OFF. Turning ON adds syntax.
pub const PARSER_CFLAGS: &[(&str, &str, &str)] = &[
    // OMIT flags — each removes a SQL statement or clause from the grammar.
    ("SQLITE_OMIT_ALTERTABLE", "omit", "removes ALTER TABLE"),
    ("SQLITE_OMIT_ANALYZE", "omit", "removes ANALYZE"),
    ("SQLITE_OMIT_ATTACH", "omit", "removes ATTACH and DETACH"),
    (
        "SQLITE_OMIT_AUTOINCREMENT",
        "omit",
        "removes AUTOINCREMENT behavior",
    ),
    ("SQLITE_OMIT_CAST", "omit", "removes CAST operator"),
    (
        "SQLITE_OMIT_COMPOUND_SELECT",
        "omit",
        "removes UNION, UNION ALL, INTERSECT, EXCEPT",
    ),
    (
        "SQLITE_OMIT_CTE",
        "omit",
        "removes common table expressions (WITH)",
    ),
    ("SQLITE_OMIT_EXPLAIN", "omit", "removes EXPLAIN"),
    (
        "SQLITE_OMIT_FOREIGN_KEY",
        "omit",
        "removes foreign key constraint syntax",
    ),
    (
        "SQLITE_OMIT_GENERATED_COLUMNS",
        "omit",
        "removes generated column syntax",
    ),
    ("SQLITE_OMIT_PRAGMA", "omit", "removes PRAGMA command"),
    ("SQLITE_OMIT_REINDEX", "omit", "removes REINDEX"),
    ("SQLITE_OMIT_RETURNING", "omit", "removes RETURNING clause"),
    (
        "SQLITE_OMIT_SUBQUERY",
        "omit",
        "removes sub-selects and IN() operator",
    ),
    (
        "SQLITE_OMIT_TEMPDB",
        "omit",
        "removes TEMP/TEMPORARY tables",
    ),
    (
        "SQLITE_OMIT_TRIGGER",
        "omit",
        "removes CREATE TRIGGER and DROP TRIGGER",
    ),
    ("SQLITE_OMIT_VACUUM", "omit", "removes VACUUM"),
    (
        "SQLITE_OMIT_VIEW",
        "omit",
        "removes CREATE VIEW and DROP VIEW",
    ),
    (
        "SQLITE_OMIT_VIRTUALTABLE",
        "omit",
        "removes virtual table mechanism",
    ),
    ("SQLITE_OMIT_WINDOWFUNC", "omit", "removes window functions"),
    // ENABLE flags — each adds syntax to the grammar.
    (
        "SQLITE_ENABLE_ORDERED_SET_AGGREGATES",
        "enable",
        "adds WITHIN keyword for ordered-set aggregates",
    ),
    (
        "SQLITE_ENABLE_UPDATE_DELETE_LIMIT",
        "enable",
        "adds ORDER BY and LIMIT on UPDATE/DELETE",
    ),
];

// ---------------------------------------------------------------------------
// Keyword cflag extraction
// ---------------------------------------------------------------------------

/// Top-level keyword cflag catalog.
#[derive(Debug, Clone, serde::Serialize)]
struct KeywordCflags {
    keywords: Vec<KeywordCflagEntry>,
}

/// A single keyword entry in the catalog.
#[derive(Debug, Clone, serde::Serialize)]
struct KeywordCflagEntry {
    name: String,
    cflag: u32,
    /// 0 = OMIT (keyword disabled when flag set), 1 = ENABLE (keyword enabled when flag set).
    polarity: u8,
}

/// Extract keyword cflag data from mkkeywordhash.c source.
///
/// Returns a map of keyword name → (cflag_index, polarity).
pub fn extract_keyword_cflags(
    mkkeywordhash_source: &str,
) -> Result<HashMap<String, (u32, u8)>, String> {
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
            if let Some(cflag_val) = super::synq_cflag_for_sqlite_flag(omit_flag) {
                map.insert(kw.name.clone(), (cflag_val, polarity));
            }
        }
    }

    Ok(map)
}

/// Write keyword cflag data to a JSON file.
pub fn write_keyword_cflags(
    cflags: &HashMap<String, (u32, u8)>,
    output_path: &Path,
) -> Result<(), String> {
    let mut entries: Vec<_> = cflags
        .iter()
        .map(|(name, (cflag, polarity))| KeywordCflagEntry {
            name: name.clone(),
            cflag: *cflag,
            polarity: *polarity,
        })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    let catalog = KeywordCflags { keywords: entries };
    let json = serde_json::to_string_pretty(&catalog)
        .map_err(|e| format!("serializing keyword cflags: {e}"))?;
    fs::write(output_path, format!("{json}\n"))
        .map_err(|e| format!("writing {}: {e}", output_path.display()))
}

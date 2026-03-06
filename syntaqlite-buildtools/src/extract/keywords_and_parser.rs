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


// ---------------------------------------------------------------------------
// Keyword cflag extraction
// ---------------------------------------------------------------------------

/// Top-level keyword cflag catalog, grouped by cflag category.
///
/// Each key is a category name (e.g. `"parser"`); the value is the list of keywords
/// gated by flags in that category, using group-local cflag indices.
#[derive(Debug, Clone, serde::Serialize)]
struct KeywordCflags {
    parser: Vec<KeywordCflagEntry>,
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
/// Returns a map of keyword name → (`cflag_index`, polarity).
///
/// # Errors
///
/// Returns an error if the keyword table cannot be parsed.
pub(crate) fn extract_keyword_cflags(
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
        if let Some(&(omit_flag, polarity)) = mask_lookup.get(kw.mask_expr.as_str())
            && let Some(local_idx) = super::group_local_index("parser", omit_flag)
        {
            map.insert(kw.name.clone(), (local_idx, polarity));
        }
    }

    Ok(map)
}

/// Write keyword cflag data to a JSON file.
///
/// # Errors
///
/// Returns an error if serialization or file writing fails.
pub(crate) fn write_keyword_cflags<S: std::hash::BuildHasher>(
    cflags: &HashMap<String, (u32, u8), S>,
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

    let catalog = KeywordCflags { parser: entries };
    let json = serde_json::to_string_pretty(&catalog)
        .map_err(|e| format!("serializing keyword cflags: {e}"))?;
    fs::write(output_path, format!("{json}\n"))
        .map_err(|e| format!("writing {}: {e}", output_path.display()))
}

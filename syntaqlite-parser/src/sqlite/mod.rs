// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! SQLite-dialect-specific data that lives in the parser crate.
//!
//! This includes compile-flag metadata, function catalogs, and helper
//! functions for cflag/version parsing.

pub mod cflag_versions_table;
pub mod functions;
pub mod functions_catalog;

use crate::dialect::ffi::CflagInfo;

/// All known compile-time flags, built once from the generated table.
///
/// Returns a static slice of [`CflagInfo`] entries in index order.
pub fn cflag_table() -> &'static [CflagInfo] {
    use std::sync::LazyLock;
    static TABLE: LazyLock<Vec<CflagInfo>> = LazyLock::new(|| {
        cflag_versions_table::CFLAG_TABLE
            .iter()
            .map(|(suffix, index, min_version, category)| CflagInfo {
                suffix: suffix.to_string(),
                index: *index,
                min_version: *min_version,
                category: category.to_string(),
            })
            .collect()
    });
    &TABLE
}

/// Parse a dotted SQLite version string (e.g. `"3.35.0"`) into an integer
/// using SQLite's encoding: `major * 1_000_000 + minor * 1_000 + patch`.
/// The string `"latest"` maps to `i32::MAX`.
pub fn parse_sqlite_version(s: &str) -> Result<i32, String> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("latest") {
        return Ok(i32::MAX);
    }
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("expected 'major.minor.patch', got '{s}'"));
    }
    let major: i32 = parts[0]
        .parse()
        .map_err(|_| format!("invalid major version: '{}'", parts[0]))?;
    let minor: i32 = parts[1]
        .parse()
        .map_err(|_| format!("invalid minor version: '{}'", parts[1]))?;
    let patch: i32 = parts[2]
        .parse()
        .map_err(|_| format!("invalid patch version: '{}'", parts[2]))?;
    Ok(major * 1_000_000 + minor * 1_000 + patch)
}

/// Look up a cflag by its full canonical name
/// (e.g. `"SYNTAQLITE_CFLAG_SQLITE_OMIT_WINDOWFUNC"`).
///
/// Returns the bit index on success.
pub fn parse_cflag_name(s: &str) -> Result<u32, String> {
    let suffix = s
        .strip_prefix("SYNTAQLITE_CFLAG_")
        .ok_or_else(|| format!("cflag name must start with 'SYNTAQLITE_CFLAG_', got '{s}'"))?;
    for entry in cflag_table() {
        if entry.suffix == suffix {
            return Ok(entry.index);
        }
    }
    Err(format!("unknown cflag: '{s}'"))
}

/// Returns all known cflag suffixes (e.g. `"SQLITE_OMIT_WINDOWFUNC"`).
pub fn cflag_names() -> Vec<&'static str> {
    cflag_table().iter().map(|e| e.suffix.as_str()).collect()
}

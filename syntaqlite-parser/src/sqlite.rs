// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! SQLite-dialect-specific data that lives in the parser crate.
//!
//! This includes compile-flag metadata, function catalogs, and helper
//! functions for cflag/version parsing.

use crate::catalog;
use crate::catalog::FunctionInfo;
use crate::dialect::ffi::CflagInfo;
use crate::DialectEnv;

// ── Built-in function catalog ────────────────────────────────────────────────

/// Returns all SQLite built-in functions available for the given config.
///
/// Filters the full catalog by version and cflags. A function is included
/// if at least one of its availability rules matches the config.
pub fn available_functions(env: &DialectEnv<'_>) -> Vec<&'static FunctionInfo<'static>> {
    crate::functions_catalog::SQLITE_FUNCTIONS
        .iter()
        .filter(|entry| catalog::is_function_available(entry, env))
        .map(|entry| &entry.info)
        .collect()
}

/// Returns the full unfiltered catalog of all SQLite built-in functions.
#[cfg(test)]
pub(crate) fn catalog() -> &'static [crate::catalog::FunctionEntry<'static>] {
    crate::functions_catalog::SQLITE_FUNCTIONS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::ffi::Cflags;
    use crate::dialect::DialectEnv;

    #[test]
    fn catalog_is_not_empty() {
        assert!(!catalog().is_empty());
        assert!(catalog().len() > 100);
    }

    #[test]
    fn default_config_returns_baseline_functions() {
        let env = DialectEnv::for_testing(i32::MAX, Cflags::new());
        let funcs = available_functions(&env);
        let names: Vec<&str> = funcs.iter().map(|f| f.name).collect();
        assert!(names.contains(&"abs"), "abs should be baseline");
        assert!(names.contains(&"count"), "count should be baseline");
        assert!(
            !names.contains(&"acos"),
            "acos requires ENABLE_MATH_FUNCTIONS"
        );
    }

    #[test]
    fn enable_math_functions_adds_math() {
        let mut cflags = Cflags::new();
        // SQLITE_ENABLE_MATH_FUNCTIONS is cflag index 34
        cflags.set(34);
        let env = DialectEnv::for_testing(i32::MAX, cflags);
        let funcs = available_functions(&env);
        let names: Vec<&str> = funcs.iter().map(|f| f.name).collect();
        assert!(
            names.contains(&"acos"),
            "acos should be available with ENABLE_MATH_FUNCTIONS"
        );
        assert!(names.contains(&"abs"), "abs should still be available");
    }

    #[test]
    fn omit_json_removes_json_functions() {
        let mut cflags = Cflags::new();
        // SQLITE_OMIT_JSON is cflag index 13
        cflags.set(13);
        let env = DialectEnv::for_testing(i32::MAX, cflags);
        let funcs = available_functions(&env);
        let names: Vec<&str> = funcs.iter().map(|f| f.name).collect();
        assert!(
            !names.contains(&"json_array"),
            "json_array should be omitted with OMIT_JSON"
        );
    }

    #[test]
    fn version_filtering_works() {
        let env = DialectEnv::for_testing(3_030_001, Cflags::new()); // 3.30.1
        let funcs = available_functions(&env);
        let names: Vec<&str> = funcs.iter().map(|f| f.name).collect();
        assert!(names.contains(&"abs"), "abs available since 3.30.1");
        assert!(
            !names.contains(&"json_array"),
            "json_array not available at 3.30.1 without ENABLE_JSON1"
        );
    }
}

// ── Cflag table and helpers ──────────────────────────────────────────────────

/// All known compile-time flags, built once from the generated table.
///
/// Returns a static slice of [`CflagInfo`] entries in index order.
pub fn cflag_table() -> &'static [CflagInfo] {
    use std::sync::LazyLock;
    static TABLE: LazyLock<Vec<CflagInfo>> = LazyLock::new(|| {
        crate::cflag_versions::CFLAG_TABLE
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

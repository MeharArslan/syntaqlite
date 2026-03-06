// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! SQLite-specific cflag and version helpers.

use syntaqlite_syntax::util::SqliteVersion;

use crate::dialect::{is_function_available, FunctionInfo};
use crate::dialect::Dialect;
use crate::sqlite::cflag_entries::CFLAG_ENTRIES;

// ── Built-in function catalog ────────────────────────────────────────────────

/// Returns all SQLite built-in functions available for the given dialect config.
///
/// Filters the full catalog by version and cflags. A function is included
/// if at least one of its availability rules matches the config.
pub(crate) fn available_functions(dialect: &Dialect) -> Vec<&'static FunctionInfo<'static>> {
    crate::sqlite::functions_catalog::SQLITE_FUNCTIONS
        .iter()
        .filter(|entry| is_function_available(entry, dialect))
        .map(|entry| &entry.info)
        .collect()
}

// ── Cflag helpers ─────────────────────────────────────────────────────────────

/// Parse a dotted SQLite version string (e.g. `"3.35.0"`) into a [`SqliteVersion`].
/// The string `"latest"` maps to [`SqliteVersion::Latest`].
pub(crate) fn parse_sqlite_version(s: &str) -> Result<SqliteVersion, String> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("latest") {
        return Ok(SqliteVersion::Latest);
    }
    SqliteVersion::parse(s)
        .ok_or_else(|| format!("unknown or unsupported SQLite version: '{s}'"))
}

/// Look up a cflag by its full canonical name
/// (e.g. `"SYNTAQLITE_CFLAG_SQLITE_OMIT_WINDOWFUNC"`).
///
/// Returns the bit index on success.
pub(crate) fn parse_cflag_name(s: &str) -> Result<u32, String> {
    let flag_name = s
        .strip_prefix("SYNTAQLITE_CFLAG_")
        .ok_or_else(|| format!("cflag name must start with 'SYNTAQLITE_CFLAG_', got '{s}'"))?;
    for &(name, index, _, _) in CFLAG_ENTRIES {
        if name == flag_name {
            return Ok(index);
        }
    }
    Err(format!("unknown cflag: '{s}'"))
}

/// Returns an iterator over all known cflag names (e.g. `"SQLITE_OMIT_WINDOWFUNC"`).
pub(crate) fn cflag_names() -> impl Iterator<Item = &'static str> {
    CFLAG_ENTRIES.iter().map(|&(name, _, _, _)| name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_known() {
        assert_eq!(parse_sqlite_version("3.35.0"), Ok(SqliteVersion::V3_35));
    }

    #[test]
    fn parse_version_latest() {
        assert_eq!(parse_sqlite_version("latest"), Ok(SqliteVersion::Latest));
    }

    #[test]
    fn parse_version_unknown() {
        assert!(parse_sqlite_version("3.99.0").is_err());
    }

    #[test]
    fn parse_cflag_known() {
        // OMIT_WINDOWFUNC is parser flag C compact index 19 (== SqliteFlag::OmitWindowfunc = 19).
        assert_eq!(
            parse_cflag_name("SYNTAQLITE_CFLAG_SQLITE_OMIT_WINDOWFUNC"),
            Ok(19)
        );
    }

    #[test]
    fn parse_cflag_bad_prefix() {
        assert!(parse_cflag_name("SQLITE_OMIT_WINDOWFUNC").is_err());
    }

    #[test]
    fn parse_cflag_unknown() {
        assert!(parse_cflag_name("SYNTAQLITE_CFLAG_SQLITE_OMIT_NONEXISTENT").is_err());
    }

    #[test]
    fn cflag_names_count() {
        let names: Vec<_> = cflag_names().collect();
        assert_eq!(names.len(), CFLAG_ENTRIES.len());
        assert!(names.contains(&"SQLITE_OMIT_WINDOWFUNC"));
        assert!(names.contains(&"SQLITE_ENABLE_FTS5"));
    }

    #[test]
    fn available_functions_latest_includes_builtins() {
        let dialect = crate::sqlite::dialect::dialect();
        let fns = available_functions(&dialect);
        assert!(fns.iter().any(|f| f.name == "abs"));
        assert!(fns.iter().any(|f| f.name == "count"));
    }
}

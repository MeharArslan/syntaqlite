// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Cross-cutting utilities for grammar configuration and compatibility.

pub use syntaqlite_syntax::util::SqliteFlag;
pub use syntaqlite_syntax::util::SqliteVersion;
use syntaqlite_syntax::util::SqliteSyntaxFlags;

/// Full set of `SQLite` compile-time compatibility flags.
///
/// Covers all 42 known flags using a 64-bit bitset indexed by [`SqliteFlag`]
/// discriminants. Parser flags (indices 0–21) share the same bit positions as
/// the C compact `SYNQ_CFLAG_IDX_*` values, so conversion to/from
/// [`SqliteSyntaxFlags`] requires no translation table.
///
/// Use this type with [`AnyDialect::with_cflags`](crate::AnyDialect::with_cflags)
/// to filter function availability based on compile-time `SQLite` configuration.
///
/// # Example
/// ```rust,ignore
/// use syntaqlite::util::{SqliteFlag, SqliteFlags};
/// let dialect = syntaqlite::sqlite_dialect()
///     .with_cflags(SqliteFlags::default().with(SqliteFlag::EnableMathFunctions));
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SqliteFlags(pub(crate) u64);

impl SqliteFlags {
    /// Returns `true` if compatibility flag `flag` is enabled.
    #[inline]
    pub fn has(&self, flag: SqliteFlag) -> bool {
        let idx = flag as u32;
        (self.0 >> idx) & 1 != 0
    }

    /// Returns `true` if the flag at raw bit-index `idx` is enabled.
    ///
    /// Use this when `idx` comes from an external source (e.g. a generated
    /// availability rule) and no [`SqliteFlag`] variant is available at the
    /// call site.
    #[inline]
    pub fn has_index(&self, idx: u32) -> bool {
        idx < 64 && (self.0 >> idx) & 1 != 0
    }

    /// Return a copy of these flags with `flag` enabled.
    #[must_use]
    pub fn with(mut self, flag: SqliteFlag) -> Self {
        self.0 |= 1u64 << (flag as u32);
        self
    }

    /// Return a copy of these flags with `flag` disabled.
    #[must_use]
    pub fn without(mut self, flag: SqliteFlag) -> Self {
        self.0 &= !(1u64 << (flag as u32));
        self
    }
}

// ── Conversion between SqliteFlags and SqliteSyntaxFlags ─────────────────────
//
// Parser flags occupy indices 0–21 in both representations (SqliteFlag
// discriminants == C compact SYNQ_CFLAG_IDX_* values). Non-parser flags
// (indices 22–41) have no CCflags representation and are dropped on
// conversion to SqliteSyntaxFlags.

impl From<SqliteFlags> for SqliteSyntaxFlags {
    /// Convert full Rust flags to C-compact parser flags.
    ///
    /// Only parser flags (indices 0–21) are preserved; non-parser flags are
    /// silently dropped since they have no C parser representation.
    fn from(flags: SqliteFlags) -> Self {
        // Parser flags are bits 0–21; CCflags covers 0–23 (3 bytes).
        // Since SqliteFlag discriminants == C compact indices for parser flags,
        // we can copy the lower 22 bits directly.
        let mut syntax = SqliteSyntaxFlags::default();
        for idx in 0..22u32 {
            if flags.has_index(idx) {
                syntax = syntax.with_compact(idx);
            }
        }
        syntax
    }
}

impl From<SqliteSyntaxFlags> for SqliteFlags {
    /// Convert C-compact parser flags to full Rust flags.
    fn from(syntax: SqliteSyntaxFlags) -> Self {
        // C compact bits 0–21 map directly to Rust global bits 0–21.
        let mut flags = SqliteFlags::default();
        for idx in 0..22u32 {
            if syntax.has_compact(idx) {
                flags.0 |= 1u64 << idx;
            }
        }
        flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::cflag_entries::CFLAG_ENTRIES;

    /// Verify that parser-category flags in CFLAG_ENTRIES all have bit_index < 22,
    /// and non-parser-category flags all have bit_index >= 22.
    ///
    /// This enforces the index invariant: SqliteFlag discriminants 0–21 ==
    /// C compact SYNQ_CFLAG_IDX_* values, so no translation table is needed.
    #[test]
    fn cflag_index_invariant() {
        for &(name, bit_index, _, categories) in CFLAG_ENTRIES {
            if categories.contains(&"parser") {
                assert!(
                    bit_index < 22,
                    "parser flag {name} has bit_index {bit_index}, expected < 22"
                );
            } else {
                assert!(
                    bit_index >= 22,
                    "non-parser flag {name} (categories={categories:?}) has bit_index {bit_index}, expected >= 22"
                );
            }
        }
    }

    /// Verify that flags with bit_index < 22 survive round-trip through
    /// SqliteSyntaxFlags with the same bit position.
    ///
    /// This is the core invariant: SqliteFlag discriminants 0–21 equal the
    /// C compact SYNQ_CFLAG_IDX_* values, so no translation table is needed.
    #[test]
    fn c_parser_flags_round_trip_through_syntax_flags() {
        for &(name, bit_index, _, _) in CFLAG_ENTRIES {
            if bit_index >= 22 {
                continue;
            }
            let rust_flags = SqliteFlags(1u64 << bit_index);
            let syntax: SqliteSyntaxFlags = rust_flags.into();
            assert!(
                syntax.has_compact(bit_index),
                "C-parser flag {name} (index {bit_index}) lost after SqliteFlags -> SqliteSyntaxFlags"
            );
            let back: SqliteFlags = syntax.into();
            assert!(
                back.has_index(bit_index),
                "C-parser flag {name} (index {bit_index}) lost after SqliteSyntaxFlags -> SqliteFlags"
            );
        }
    }

    /// Verify that flags with bit_index >= 22 are dropped when converting to
    /// SqliteSyntaxFlags (they have no C compact representation).
    #[test]
    fn rust_only_flags_dropped_in_syntax_flags() {
        for &(name, bit_index, _, _) in CFLAG_ENTRIES {
            if bit_index < 22 {
                continue;
            }
            let rust_flags = SqliteFlags(1u64 << bit_index);
            let syntax: SqliteSyntaxFlags = rust_flags.into();
            assert!(
                !syntax.has_compact(bit_index),
                "Rust-only flag {name} (index {bit_index}) should be absent from SqliteSyntaxFlags"
            );
        }
    }
}

// ── Built-in function catalog ────────────────────────────────────────────────

/// Returns all `SQLite` built-in functions available for the given dialect config.
///
/// Filters the full catalog by version and cflags. A function is included
/// if at least one of its availability rules matches the config.
#[cfg(all(feature = "fmt", feature = "sqlite"))]
pub(crate) fn available_functions(
    dialect: &crate::dialect::Dialect,
) -> Vec<&'static crate::dialect::FunctionInfo<'static>> {
    crate::sqlite::functions_catalog::SQLITE_FUNCTIONS
        .iter()
        .filter(|entry| crate::dialect::is_function_available(entry, dialect))
        .map(|entry| &entry.info)
        .collect()
}

// ── Cflag helpers ─────────────────────────────────────────────────────────────

/// Parse a dotted `SQLite` version string (e.g. `"3.35.0"`) into a [`SqliteVersion`].
/// The string `"latest"` maps to [`SqliteVersion::Latest`].
pub fn parse_sqlite_version(s: &str) -> Result<SqliteVersion, String> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("latest") {
        return Ok(SqliteVersion::Latest);
    }
    SqliteVersion::parse(s).ok_or_else(|| format!("unknown or unsupported SQLite version: '{s}'"))
}

/// Look up a cflag by its full canonical name
/// (e.g. `"SYNTAQLITE_CFLAG_SQLITE_OMIT_WINDOWFUNC"`).
///
/// Returns the bit index on success.
#[cfg(feature = "sqlite")]
pub(crate) fn parse_cflag_name(s: &str) -> Result<u32, String> {
    use crate::sqlite::cflag_entries::CFLAG_ENTRIES;
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
#[cfg(feature = "sqlite")]
pub(crate) fn cflag_names() -> impl Iterator<Item = &'static str> {
    use crate::sqlite::cflag_entries::CFLAG_ENTRIES;
    CFLAG_ENTRIES.iter().map(|&(name, _, _, _)| name)
}

#[cfg(test)]
#[cfg(feature = "sqlite")]
mod cflag_tests {
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
        use crate::sqlite::cflag_entries::CFLAG_ENTRIES;
        let names: Vec<_> = cflag_names().collect();
        assert_eq!(names.len(), CFLAG_ENTRIES.len());
        assert!(names.contains(&"SQLITE_OMIT_WINDOWFUNC"));
        assert!(names.contains(&"SQLITE_ENABLE_FTS5"));
    }

    #[test]
    #[cfg(feature = "fmt")]
    fn available_functions_latest_includes_builtins() {
        let dialect = crate::sqlite::dialect::dialect();
        let fns = available_functions(&dialect);
        assert!(fns.iter().any(|f| f.name == "abs"));
        assert!(fns.iter().any(|f| f.name == "count"));
    }
}

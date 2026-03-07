// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Cross-cutting utilities for grammar configuration and compatibility.

pub use crate::sqlite::cflags::SqliteFlag;
pub use syntaqlite_syntax::util::{SqliteSyntaxFlag, SqliteSyntaxFlags, SqliteVersion};

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
        (self.0 >> (flag as u32)) & 1 != 0
    }

    /// Returns `true` if the flag at raw bit-index `idx` is enabled.
    ///
    /// Internal helper for generated availability rules where no
    /// [`SqliteFlag`] variant is available at the call site.
    #[inline]
    pub(crate) fn has_index(&self, idx: u32) -> bool {
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
// Parser flags occupy indices 0–21 in both representations.
// SqliteFlag::as_syntax_flag() maps each parser flag to its SqliteSyntaxFlag
// counterpart; non-parser flags return None and are silently dropped.

impl From<SqliteFlags> for SqliteSyntaxFlags {
    /// Convert full Rust flags to C-compact parser flags.
    ///
    /// Only parser flags (indices 0–21) are preserved; non-parser flags are
    /// silently dropped since they have no C parser representation.
    fn from(flags: SqliteFlags) -> Self {
        let mut s = SqliteSyntaxFlags::default();
        for &flag in SqliteFlag::all() {
            if let Some(sf) = flag.as_syntax_flag()
                && flags.has(flag) {
                    s = s.with(sf);
                }
        }
        s
    }
}

impl From<SqliteSyntaxFlags> for SqliteFlags {
    /// Convert C-compact parser flags to full Rust flags.
    fn from(s: SqliteSyntaxFlags) -> Self {
        let mut flags = SqliteFlags::default();
        for &sf in SqliteSyntaxFlag::all() {
            if s.has(sf) {
                // Parser flags 0–21 have identical discriminants in both enums.
                flags.0 |= 1u64 << (sf as u32);
            }
        }
        flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that parser-category flags all have discriminant < 22,
    /// and non-parser flags all have discriminant >= 22.
    #[test]
    fn cflag_index_invariant() {
        for &flag in SqliteFlag::all() {
            let idx = flag as u32;
            if flag.categories().contains(&"parser") {
                assert!(idx < 22, "parser flag {} has index {idx}, expected < 22", flag.name());
            } else {
                assert!(
                    idx >= 22,
                    "non-parser flag {} has index {idx}, expected >= 22",
                    flag.name()
                );
            }
        }
    }

    /// Verify that parser flags survive round-trip through `SqliteSyntaxFlags`.
    #[test]
    fn c_parser_flags_round_trip_through_syntax_flags() {
        for &flag in SqliteFlag::all() {
            if !flag.is_parser_flag() {
                continue;
            }
            let bit_index = flag as u32;
            let rust_flags = SqliteFlags(1u64 << bit_index);
            let syntax: SqliteSyntaxFlags = rust_flags.into();
            let back: SqliteFlags = syntax.into();
            assert!(
                back.has_index(bit_index),
                "C-parser flag {} (index {bit_index}) lost in SqliteFlags -> SqliteSyntaxFlags -> SqliteFlags round-trip",
                flag.name()
            );
        }
    }

    /// Verify that non-parser flags are dropped when converting to `SqliteSyntaxFlags`.
    #[test]
    fn rust_only_flags_dropped_in_syntax_flags() {
        for &flag in SqliteFlag::all() {
            if flag.is_parser_flag() {
                continue;
            }
            let bit_index = flag as u32;
            let rust_flags = SqliteFlags(1u64 << bit_index);
            let syntax: SqliteSyntaxFlags = rust_flags.into();
            let back: SqliteFlags = syntax.into();
            assert!(
                !back.has_index(bit_index),
                "Rust-only flag {} (index {bit_index}) should be absent after round-trip through SqliteSyntaxFlags",
                flag.name()
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

#[cfg(test)]
#[cfg(feature = "sqlite")]
mod cflag_tests {
    use super::*;

    #[test]
    fn parse_version_known() {
        assert_eq!(SqliteVersion::parse_with_latest("3.35.0"), Ok(SqliteVersion::V3_35));
    }

    #[test]
    fn parse_version_latest() {
        assert_eq!(SqliteVersion::parse_with_latest("latest"), Ok(SqliteVersion::Latest));
    }

    #[test]
    fn parse_version_unknown() {
        assert!(SqliteVersion::parse_with_latest("3.99.0").is_err());
    }

    #[test]
    fn parse_cflag_known() {
        assert_eq!(
            SqliteFlag::from_prefixed_name("SYNTAQLITE_CFLAG_SQLITE_OMIT_WINDOWFUNC"),
            Ok(SqliteFlag::OmitWindowfunc)
        );
    }

    #[test]
    fn parse_cflag_bad_prefix() {
        assert!(SqliteFlag::from_prefixed_name("SQLITE_OMIT_WINDOWFUNC").is_err());
    }

    #[test]
    fn parse_cflag_unknown() {
        assert!(SqliteFlag::from_prefixed_name("SYNTAQLITE_CFLAG_SQLITE_OMIT_NONEXISTENT").is_err());
    }

    #[test]
    fn cflag_names_count() {
        let names: Vec<_> = SqliteFlag::all().iter().map(|f| f.name()).collect();
        assert_eq!(names.len(), SqliteFlag::all().len());
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

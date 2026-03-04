// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

/// A supported SQLite major.minor version.
///
/// Used to constrain a grammar to a specific SQLite release, enabling
/// dead-code elimination of grammar rules introduced in later versions.
///
/// Patch versions are ignored — `"3.35.5"` and `"3.35.0"` both map to
/// [`V3_35`](SqliteVersion::V3_35).
///
/// Versions correspond to the `SOURCE_VERSIONS` list in
/// `python/tools/sqlite_data.py` (3.12 through 3.51).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SqliteVersion {
    V3_12,
    V3_13,
    V3_14,
    V3_15,
    V3_16,
    V3_17,
    V3_18,
    V3_19,
    V3_20,
    V3_21,
    V3_22,
    V3_23,
    V3_24,
    V3_25,
    V3_26,
    V3_27,
    V3_28,
    V3_29,
    V3_30,
    V3_31,
    V3_32,
    V3_33,
    V3_34,
    V3_35,
    V3_36,
    V3_37,
    V3_38,
    V3_39,
    V3_40,
    V3_41,
    V3_42,
    V3_43,
    V3_44,
    V3_45,
    V3_46,
    V3_47,
    V3_48,
    V3_49,
    V3_50,
    V3_51,
    /// No version constraint — use the latest grammar rules.
    Latest,
}

impl SqliteVersion {
    /// Parse a version string, ignoring the patch component.
    ///
    /// Accepts `"3.35"`, `"3.35.0"`, `"3.35.5"`, etc.
    /// Returns `None` if the version is not in the supported range.
    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s.splitn(3, '.');
        let major: u32 = parts.next()?.parse().ok()?;
        let minor: u32 = parts.next()?.parse().ok()?;
        // patch component is ignored
        if major != 3 {
            return None;
        }
        Some(match minor {
            12 => Self::V3_12,
            13 => Self::V3_13,
            14 => Self::V3_14,
            15 => Self::V3_15,
            16 => Self::V3_16,
            17 => Self::V3_17,
            18 => Self::V3_18,
            19 => Self::V3_19,
            20 => Self::V3_20,
            21 => Self::V3_21,
            22 => Self::V3_22,
            23 => Self::V3_23,
            24 => Self::V3_24,
            25 => Self::V3_25,
            26 => Self::V3_26,
            27 => Self::V3_27,
            28 => Self::V3_28,
            29 => Self::V3_29,
            30 => Self::V3_30,
            31 => Self::V3_31,
            32 => Self::V3_32,
            33 => Self::V3_33,
            34 => Self::V3_34,
            35 => Self::V3_35,
            36 => Self::V3_36,
            37 => Self::V3_37,
            38 => Self::V3_38,
            39 => Self::V3_39,
            40 => Self::V3_40,
            41 => Self::V3_41,
            42 => Self::V3_42,
            43 => Self::V3_43,
            44 => Self::V3_44,
            45 => Self::V3_45,
            46 => Self::V3_46,
            47 => Self::V3_47,
            48 => Self::V3_48,
            49 => Self::V3_49,
            50 => Self::V3_50,
            51 => Self::V3_51,
            _ => return None,
        })
    }

    /// Convert to SQLite's `SQLITE_VERSION_NUMBER` integer format.
    ///
    /// Uses the formula `major * 1_000_000 + minor * 1_000`, matching the
    /// `SQLITE_VERSION_NUMBER` C macro (e.g. `V3_35` → `3035000`).
    pub(crate) fn as_int(self) -> i32 {
        match self {
            Self::V3_12 => 3012000,
            Self::V3_13 => 3013000,
            Self::V3_14 => 3014000,
            Self::V3_15 => 3015000,
            Self::V3_16 => 3016000,
            Self::V3_17 => 3017000,
            Self::V3_18 => 3018000,
            Self::V3_19 => 3019000,
            Self::V3_20 => 3020000,
            Self::V3_21 => 3021000,
            Self::V3_22 => 3022000,
            Self::V3_23 => 3023000,
            Self::V3_24 => 3024000,
            Self::V3_25 => 3025000,
            Self::V3_26 => 3026000,
            Self::V3_27 => 3027000,
            Self::V3_28 => 3028000,
            Self::V3_29 => 3029000,
            Self::V3_30 => 3030000,
            Self::V3_31 => 3031000,
            Self::V3_32 => 3032000,
            Self::V3_33 => 3033000,
            Self::V3_34 => 3034000,
            Self::V3_35 => 3035000,
            Self::V3_36 => 3036000,
            Self::V3_37 => 3037000,
            Self::V3_38 => 3038000,
            Self::V3_39 => 3039000,
            Self::V3_40 => 3040000,
            Self::V3_41 => 3041000,
            Self::V3_42 => 3042000,
            Self::V3_43 => 3043000,
            Self::V3_44 => 3044000,
            Self::V3_45 => 3045000,
            Self::V3_46 => 3046000,
            Self::V3_47 => 3047000,
            Self::V3_48 => 3048000,
            Self::V3_49 => 3049000,
            Self::V3_50 => 3050000,
            Self::V3_51 => 3051000,
            Self::Latest => i32::MAX,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_major_minor() {
        assert_eq!(SqliteVersion::parse("3.35"), Some(SqliteVersion::V3_35));
    }

    #[test]
    fn parse_with_patch() {
        assert_eq!(SqliteVersion::parse("3.35.5"), Some(SqliteVersion::V3_35));
    }

    #[test]
    fn parse_unknown_minor() {
        assert_eq!(SqliteVersion::parse("3.99"), None);
    }

    #[test]
    fn as_int_spot_check() {
        assert_eq!(SqliteVersion::V3_35.as_int(), 3035000);
        assert_eq!(SqliteVersion::V3_51.as_int(), 3051000);
        assert_eq!(SqliteVersion::Latest.as_int(), i32::MAX);
    }
}

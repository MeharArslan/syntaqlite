// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! C ABI mirror structs for the dialect.

pub const FIELD_NODE_ID: u8 = 0;
pub const FIELD_SPAN: u8 = 1;
pub const FIELD_BOOL: u8 = 2;
pub const FIELD_FLAGS: u8 = 3;
pub const FIELD_ENUM: u8 = 4;

/// Mirrors C `SyntaqliteCflags` from `include/syntaqlite/sqlite_cflags.h`.
///
/// A packed bitfield struct. On the Rust side we represent it as raw bytes
/// and provide index-based accessors matching the C `SYNQ_CFLAG_IDX_*` constants.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Cflags {
    bytes: [u8; 6],
}

impl Cflags {
    /// Create a zero-initialized (all flags off) cflags.
    pub const fn new() -> Self {
        Self { bytes: [0; 6] }
    }

    /// Check if cflag at `idx` is set.
    #[inline]
    pub fn has(&self, idx: u32) -> bool {
        let byte = idx / 8;
        let bit = idx % 8;
        (byte < 6) && (self.bytes[byte as usize] >> bit) & 1 != 0
    }

    /// Set cflag at `idx`.
    #[inline]
    pub fn set(&mut self, idx: u32) {
        let byte = idx / 8;
        let bit = idx % 8;
        if byte < 6 {
            self.bytes[byte as usize] |= 1 << bit;
        }
    }

    /// Clear cflag at `idx`.
    #[inline]
    pub fn clear(&mut self, idx: u32) {
        let byte = idx / 8;
        let bit = idx % 8;
        if byte < 6 {
            self.bytes[byte as usize] &= !(1 << bit);
        }
    }

    /// Reset all cflags to zero.
    #[inline]
    pub fn clear_all(&mut self) {
        self.bytes = [0; 6];
    }
}

impl Default for Cflags {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Cflags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cflags({:02x?})", &self.bytes)
    }
}

// ── Cflag metadata table ─────────────────────────────────────────────

/// The raw content of `sqlite_cflags.h`, embedded at compile time.
const CFLAGS_HEADER: &str = include_str!("../../include/syntaqlite/sqlite_cflags.h");

/// Metadata for a single compile-time flag.
#[derive(Debug, Clone)]
pub struct CflagInfo {
    /// The suffix shared across C and Rust (e.g. `"SQLITE_OMIT_WINDOWFUNC"`).
    ///
    /// - C define: `SYNTAQLITE_CFLAG_SQLITE_OMIT_WINDOWFUNC`
    /// - Rust env var: `SYNTAQLITE_CFLAG_SQLITE_OMIT_WINDOWFUNC=1`
    pub suffix: String,
    /// Bit index in [`Cflags`] (matches `SYNQ_CFLAG_IDX_*` constants).
    pub index: u32,
    /// Minimum SQLite version (encoded integer) at which this cflag has any
    /// observable effect on keyword recognition. Zero means baseline (all versions).
    pub min_version: i32,
    /// Feature-area category for UI grouping:
    /// `"parser"`, `"functions"`, `"vtable"`, or `"extensions"`.
    pub category: String,
}

// Generated table mapping cflag names to minimum SQLite versions.
include!("cflag_versions_table.rs");

/// All known compile-time flags, parsed once from the embedded C header.
///
/// Returns a static slice of [`CflagInfo`] entries in index order.
pub fn cflag_table() -> &'static [CflagInfo] {
    use std::sync::LazyLock;
    static TABLE: LazyLock<Vec<CflagInfo>> = LazyLock::new(|| {
        // Collect SYNQ_CFLAG_IDX_* defines from the header.
        let mut entries = Vec::new();
        for line in CFLAGS_HEADER.lines() {
            let Some(rest) = line.strip_prefix("#define SYNQ_CFLAG_IDX_") else {
                continue;
            };
            let mut parts = rest.split_whitespace();
            let Some(raw_suffix) = parts.next() else {
                continue;
            };
            if raw_suffix == "COUNT" {
                continue;
            }
            let Some(idx_str) = parts.next() else {
                continue;
            };
            let Ok(index) = idx_str.parse::<u32>() else {
                continue;
            };
            // Prepend "SQLITE_" so suffixes match SQLite define names
            // (e.g. "OMIT_WINDOWFUNC" → "SQLITE_OMIT_WINDOWFUNC").
            let suffix = format!("SQLITE_{raw_suffix}");
            // Look up min_version and category from the generated table.
            let (min_version, category) = CFLAG_VERSIONS
                .iter()
                .find(|(name, _, _)| *name == suffix)
                .map(|(_, ver, cat)| (*ver, cat.to_string()))
                .unwrap_or((0, "parser".to_string()));
            entries.push(CflagInfo {
                suffix,
                index,
                min_version,
                category,
            });
        }
        entries
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

/// Mirrors C `SyntaqliteDialectConfig` from `include/syntaqlite/dialect_config.h`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DialectConfig {
    pub sqlite_version: i32,
    pub cflags: Cflags,
}

impl Default for DialectConfig {
    fn default() -> Self {
        Self {
            sqlite_version: i32::MAX,
            cflags: Cflags::new(),
        }
    }
}

// ── Function extension FFI mirrors ─────────────────────────────────────

/// Mirrors C `SyntaqliteFunctionInfo` from `include/syntaqlite/dialect.h`.
#[repr(C)]
pub struct FunctionInfoC {
    pub name: *const std::ffi::c_char,
    pub arities: *const i16,
    pub arity_count: u16,
    pub category: u8,
}

/// Mirrors C `SyntaqliteAvailabilityRule` from `include/syntaqlite/dialect.h`.
#[repr(C)]
pub struct AvailabilityRuleC {
    pub since: i32,
    pub until: i32,
    pub cflag_index: u32,
    pub cflag_polarity: u8,
}

/// Mirrors C `SyntaqliteFunctionEntry` from `include/syntaqlite/dialect.h`.
#[repr(C)]
pub struct FunctionEntryC {
    pub info: FunctionInfoC,
    pub availability: *const AvailabilityRuleC,
    pub availability_count: u16,
}

// Layout assertions for FFI mirrors.
const _: () = {
    assert!(std::mem::size_of::<AvailabilityRuleC>() == 16);
};

/// Mirrors C `SyntaqliteFieldMeta` from `include/syntaqlite/dialect.h`.
#[repr(C)]
pub struct FieldMeta {
    pub offset: u16,
    pub kind: u8,
    pub name: *const std::ffi::c_char,
    pub display: *const *const std::ffi::c_char,
    pub display_count: u8,
}

/// Mirrors the C `Dialect` struct defined in `include/syntaqlite/dialect.h`.
#[repr(C)]
pub struct Dialect {
    pub name: *const std::ffi::c_char,

    // Range metadata
    pub range_meta: *const std::ffi::c_void,

    // Well-known token IDs (int32_t in C)
    pub tk_space: i32,
    pub tk_semi: i32,
    pub tk_comment: i32,

    // AST metadata
    pub node_count: u32,
    pub node_names: *const *const std::ffi::c_char,
    pub field_meta: *const *const FieldMeta,
    pub field_meta_counts: *const u8,
    pub list_tags: *const u8,

    // Formatter data
    pub fmt_strings: *const *const std::ffi::c_char,
    pub fmt_string_lens: *const u16,
    pub fmt_string_count: u16,
    pub fmt_enum_display: *const u16,
    pub fmt_enum_display_count: u16,
    pub fmt_ops: *const u8,
    pub fmt_op_count: u16,
    pub fmt_dispatch: *const u32,
    pub fmt_dispatch_count: u16,

    // Parser lifecycle (function pointers provided by dialect)
    pub parser_alloc: *const std::ffi::c_void,
    pub parser_init: *const std::ffi::c_void,
    pub parser_finalize: *const std::ffi::c_void,
    pub parser_free: *const std::ffi::c_void,
    pub parser_feed: *const std::ffi::c_void,
    pub parser_trace: *const std::ffi::c_void,
    pub parser_expected_tokens: *const std::ffi::c_void,
    pub parser_completion_context: *const std::ffi::c_void,

    // Tokenizer (function pointer provided by dialect)
    pub get_token: *const std::ffi::c_void,

    // Keyword table metadata
    pub keyword_text: *const std::ffi::c_char,
    pub keyword_offsets: *const u16,
    pub keyword_lens: *const u8,
    pub keyword_codes: *const u8,
    pub keyword_count: *const u32,

    // Token metadata (indexed by token type ordinal)
    pub token_categories: *const u8,
    pub token_type_count: u32,

    // Dialect function extensions
    pub function_extensions: *const FunctionEntryC,
    pub function_extension_count: u32,
}

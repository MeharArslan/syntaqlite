// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! C ABI mirror structs for the dialect.
//!
//! All `#[repr(C)]` FFI types live in `syntaqlite_sys::dialect` and
//! are re-exported here for crate-internal use.

pub use syntaqlite_sys::dialect::*;

// ── Cflag metadata table ─────────────────────────────────────────────

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

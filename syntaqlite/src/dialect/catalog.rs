// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Core function catalog types shared across all dialects.
//!
//! These types are used by the generated `functions_catalog.rs` and by
//! dialect extensions to describe function availability.

use syntaqlite_syntax::util::SqliteVersion;

use super::handle::Dialect;

/// Category of a built-in function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FunctionCategory {
    Scalar,
    Aggregate,
    Window,
}

/// Whether a cflag enables or omits the function.
///
/// `#[repr(u8)]` with `Enable=0, Omit=1` matches the C ABI
/// (`0=Enable, 1=Omit` in `SyntaqliteAvailabilityRule.cflag_polarity`),
/// which allows [`AvailabilityRule`] to be cast directly from C data.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CflagPolarity {
    /// Function requires the cflag to be set (SQLITE_ENABLE_*).
    Enable = 0,
    /// Function is omitted when the cflag is set (SQLITE_OMIT_*).
    Omit = 1,
}

/// Metadata about a built-in function.
#[derive(Debug, Clone, Copy)]
pub struct FunctionInfo<'a> {
    /// Function name (lowercase).
    pub name: &'a str,
    /// Supported arities. Negative values indicate variadic:
    /// -1 = any number of args, -N = at least N-1 args.
    pub arities: &'a [i16],
    /// Function category.
    pub category: FunctionCategory,
}

/// A version/cflag availability rule for a function.
///
/// `#[repr(C)]` with layout `i32+i32+u32+u8+3pad = 16 bytes` matches
/// `SyntaqliteAvailabilityRule` exactly, so C extension data can be
/// reinterpreted as `&[AvailabilityRule]` without copying.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AvailabilityRule {
    /// Minimum SQLite version (encoded: major*1_000_000 + minor*1_000 + patch).
    pub since: i32,
    /// Maximum SQLite version (exclusive). 0 means no upper bound.
    pub until: i32,
    /// Cflag bit index, or `u32::MAX` if no cflag required.
    pub cflag_index: u32,
    /// Polarity of the cflag constraint.
    pub cflag_polarity: CflagPolarity,
}

const _: () = {
    assert!(std::mem::size_of::<AvailabilityRule>() == 16);
};

/// A function entry combining metadata with availability rules.
#[derive(Debug, Clone, Copy)]
pub struct FunctionEntry<'a> {
    pub info: FunctionInfo<'a>,
    pub availability: &'a [AvailabilityRule],
}

/// Check whether a function entry is available for the given dialect config.
///
/// Checks version constraints only; cflag constraints are ignored for now.
pub fn is_function_available(entry: &FunctionEntry<'_>, dialect: &Dialect<'_>) -> bool {
    entry.availability.iter().any(|rule| {
        if dialect.version() < SqliteVersion::from_int(rule.since) {
            return false;
        }
        if rule.until != 0 && dialect.version() >= SqliteVersion::from_int(rule.until) {
            return false;
        }
        true
    })
}

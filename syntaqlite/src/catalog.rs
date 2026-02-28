// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Core function catalog types shared across all dialects.
//!
//! These types are used by the generated `functions_catalog.rs` and by
//! dialect extensions to describe function availability.

use crate::dialect::ffi::DialectConfig;

/// Category of a built-in function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FunctionCategory {
    Scalar,
    Aggregate,
    Window,
}

/// Whether a cflag enables or omits the function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CflagPolarity {
    /// Function requires the cflag to be set (SQLITE_ENABLE_*).
    Enable,
    /// Function is omitted when the cflag is set (SQLITE_OMIT_*).
    Omit,
}

/// Metadata about a built-in function.
#[derive(Debug, Clone, Copy)]
pub struct FunctionInfo {
    /// Function name (lowercase).
    pub name: &'static str,
    /// Supported arities. Negative values indicate variadic:
    /// -1 = any number of args, -N = at least N-1 args.
    pub arities: &'static [i16],
    /// Function category.
    pub category: FunctionCategory,
}

/// A version/cflag availability rule for a function.
#[derive(Debug, Clone, Copy)]
pub struct AvailabilityRule {
    /// Minimum SQLite version (encoded: major*1_000_000 + minor*1_000 + patch).
    pub since: i32,
    /// Maximum SQLite version (exclusive). 0 means no upper bound.
    pub until: i32,
    /// Cflag bit index in `Cflags`, or `u32::MAX` if no cflag required.
    pub cflag_index: u32,
    /// Polarity of the cflag constraint.
    pub cflag_polarity: CflagPolarity,
}

/// A function entry combining metadata with availability rules.
#[derive(Debug, Clone, Copy)]
pub struct FunctionEntry {
    pub info: FunctionInfo,
    pub availability: &'static [AvailabilityRule],
}

/// Check whether a function entry is available for the given dialect config.
///
/// A function is available if *any* of its availability rules matches.
/// A rule matches when:
/// - The config version is >= `since`
/// - The config version is < `until` (if `until` is non-zero)
/// - If a cflag is required: for `Enable` polarity, the cflag must be set;
///   for `Omit` polarity, the cflag must NOT be set.
pub fn is_available(entry: &FunctionEntry, config: &DialectConfig) -> bool {
    entry.availability.iter().any(|rule| {
        if config.sqlite_version < rule.since {
            return false;
        }
        if rule.until != 0 && config.sqlite_version >= rule.until {
            return false;
        }
        if rule.cflag_index != u32::MAX {
            let flag_set = config.cflags.has(rule.cflag_index);
            match rule.cflag_polarity {
                CflagPolarity::Enable => flag_set,
                CflagPolarity::Omit => !flag_set,
            }
        } else {
            true
        }
    })
}

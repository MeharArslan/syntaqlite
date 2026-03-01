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
///
/// `#[repr(u8)]` with `Enable=0, Omit=1` matches the C ABI
/// (`0=Enable, 1=Omit` in `SyntaqliteAvailabilityRule.cflag_polarity`),
/// which allows [`AvailabilityRule`] to be cast directly from C data.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CflagPolarity {
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
pub(crate) struct AvailabilityRule {
    /// Minimum SQLite version (encoded: major*1_000_000 + minor*1_000 + patch).
    pub(crate) since: i32,
    /// Maximum SQLite version (exclusive). 0 means no upper bound.
    pub(crate) until: i32,
    /// Cflag bit index in `Cflags`, or `u32::MAX` if no cflag required.
    pub(crate) cflag_index: u32,
    /// Polarity of the cflag constraint.
    pub(crate) cflag_polarity: CflagPolarity,
}

const _: () = {
    assert!(std::mem::size_of::<AvailabilityRule>() == 16);
    assert!(
        std::mem::size_of::<AvailabilityRule>()
            == std::mem::size_of::<crate::dialect::ffi::AvailabilityRuleC>()
    );
    assert!(
        std::mem::align_of::<AvailabilityRule>()
            == std::mem::align_of::<crate::dialect::ffi::AvailabilityRuleC>()
    );
};

/// A function entry combining metadata with availability rules.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FunctionEntry<'a> {
    pub(crate) info: FunctionInfo<'a>,
    pub(crate) availability: &'a [AvailabilityRule],
}

/// Check whether a function entry is available for the given dialect config.
///
/// A function is available if *any* of its availability rules matches.
/// A rule matches when:
/// - The config version is >= `since`
/// - The config version is < `until` (if `until` is non-zero)
/// - If a cflag is required: for `Enable` polarity, the cflag must be set;
///   for `Omit` polarity, the cflag must NOT be set.
pub(crate) fn is_available(entry: &FunctionEntry<'_>, config: &DialectConfig) -> bool {
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

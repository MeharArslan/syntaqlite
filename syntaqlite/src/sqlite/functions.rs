// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! SQLite built-in function catalog with version/cflag-aware filtering.
//!
//! The catalog is generated from SQLite's source and contains all 179 built-in
//! functions with their arity, category, and availability constraints.
//!
//! # Example
//!
//! ```
//! use syntaqlite::sqlite::config::DialectConfig;
//! use syntaqlite::sqlite::functions;
//!
//! // Default config (latest version, no cflags) — baseline functions only.
//! let config = DialectConfig::default();
//! let funcs = functions::available_functions(&config);
//! assert!(funcs.len() > 60);
//! ```

#[path = "functions_catalog.rs"]
mod functions_catalog;

pub use crate::catalog::{
    AvailabilityRule, CflagPolarity, FunctionCategory, FunctionEntry, FunctionInfo,
};
pub use functions_catalog::SQLITE_FUNCTIONS;

use crate::catalog;
use crate::dialect::ffi::DialectConfig;

/// Returns all SQLite built-in functions available for the given config.
///
/// Filters the full catalog by version and cflags. A function is included
/// if at least one of its availability rules matches the config.
pub fn available_functions(config: &DialectConfig) -> Vec<&'static FunctionInfo> {
    SQLITE_FUNCTIONS
        .iter()
        .filter(|entry| catalog::is_available(entry, config))
        .map(|entry| &entry.info)
        .collect()
}

/// Returns the full unfiltered catalog of all SQLite built-in functions.
pub fn catalog() -> &'static [FunctionEntry] {
    SQLITE_FUNCTIONS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::ffi::{Cflags, DialectConfig};

    #[test]
    fn catalog_is_not_empty() {
        assert!(!catalog().is_empty());
        assert!(catalog().len() > 100);
    }

    #[test]
    fn default_config_returns_baseline_functions() {
        let config = DialectConfig::default();
        let funcs = available_functions(&config);
        // Default config has latest version and no cflags set.
        // Functions with no cflag requirement should be available.
        // Functions with ENABLE cflags should NOT be available (cflags not set).
        // Functions with OMIT cflags SHOULD be available (omit flag not set).
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
        let mut config = DialectConfig::default();
        // SQLITE_ENABLE_MATH_FUNCTIONS is cflag index 34
        config.cflags.set(34);
        let funcs = available_functions(&config);
        let names: Vec<&str> = funcs.iter().map(|f| f.name).collect();
        assert!(
            names.contains(&"acos"),
            "acos should be available with ENABLE_MATH_FUNCTIONS"
        );
        assert!(names.contains(&"abs"), "abs should still be available");
    }

    #[test]
    fn omit_json_removes_json_functions() {
        let mut config = DialectConfig::default();
        // SQLITE_OMIT_JSON is cflag index 13
        config.cflags.set(13);
        let funcs = available_functions(&config);
        let names: Vec<&str> = funcs.iter().map(|f| f.name).collect();
        assert!(
            !names.contains(&"json_array"),
            "json_array should be omitted with OMIT_JSON"
        );
    }

    #[test]
    fn version_filtering_works() {
        let config = DialectConfig {
            sqlite_version: 3_030_001, // 3.30.1
            cflags: Cflags::new(),
        };
        let funcs = available_functions(&config);
        let names: Vec<&str> = funcs.iter().map(|f| f.name).collect();
        assert!(names.contains(&"abs"), "abs available since 3.30.1");
        // json_array requires >= 3.38.5 with default (omit polarity)
        // At 3.30.1, only the ENABLE_JSON1 rule applies, which needs cflag set
        assert!(
            !names.contains(&"json_array"),
            "json_array not available at 3.30.1 without ENABLE_JSON1"
        );
    }
}

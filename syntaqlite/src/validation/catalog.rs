// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use syntaqlite_parser::dialect::ffi::DialectConfig;

use super::types::{FunctionDef, SessionContext, expand_function_info};
use crate::dialect::Dialect;

/// Resolved function catalog for a dialect configuration.
///
/// Merges three sources in priority order:
/// 1. SQLite built-in catalog (filtered by `DialectConfig`)
/// 2. Dialect extension functions (filtered by `DialectConfig`)
/// 3. Session context user-defined functions (via [`with_session`](FunctionCatalog::with_session))
pub struct FunctionCatalog {
    functions: Vec<FunctionDef>,
}

impl FunctionCatalog {
    /// Build the catalog from a dialect and its compile-time configuration.
    ///
    /// Includes the SQLite built-in catalog and dialect extensions, both
    /// filtered by `config`. Call [`with_session`](Self::with_session) to
    /// merge in user-defined functions from a session context.
    pub fn for_dialect(dialect: &Dialect<'_>, config: &DialectConfig) -> Self {
        #[cfg(feature = "sqlite")]
        let mut functions: Vec<FunctionDef> =
            syntaqlite_parser::sqlite::available_functions(config)
                .into_iter()
                .flat_map(|info| expand_function_info(info))
                .collect();

        #[cfg(not(feature = "sqlite"))]
        let mut functions: Vec<FunctionDef> = Vec::new();

        for ext in dialect.function_extensions() {
            if syntaqlite_parser::catalog::is_available(&ext, config) {
                functions.extend(expand_function_info(&ext.info));
            }
        }

        FunctionCatalog { functions }
    }

    /// Append user-defined functions from a session context.
    pub fn with_session(mut self, session: &SessionContext) -> Self {
        self.functions.extend(session.functions.iter().cloned());
        self
    }

    /// All function definitions (may contain multiple entries per name for
    /// different arities).
    pub fn functions(&self) -> &[FunctionDef] {
        &self.functions
    }

    /// Unique function names, deduplicated across arities.
    pub fn unique_names(&self) -> impl Iterator<Item = &str> {
        let mut seen = std::collections::HashSet::new();
        self.functions
            .iter()
            .filter(move |f| seen.insert(f.name.as_str()))
            .map(|f| f.name.as_str())
    }
}

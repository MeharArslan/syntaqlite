// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use syntaqlite_parser::DialectEnv;

use syntaqlite_parser::{FunctionCategory, FunctionInfo};

use super::types::{FunctionCheckResult, FunctionDef, FunctionLookup};

/// Resolved function catalog for a dialect + config combination.
///
/// Merges three sources with the following priority:
/// 1. SQLite built-in catalog (filtered by [`DialectEnv`])
/// 2. Dialect extension functions (filtered by [`DialectEnv`])
/// 3. Session/document user-defined functions
///
/// Unlike the old `Vec<FunctionDef>` approach, this does **not** expand
/// one entry per arity — arity checking works directly on the compact
/// `&[i16]` representation from `FunctionInfo`.
#[derive(Clone)]
pub struct FunctionCatalog {
    /// Built-in functions (borrowed from static catalog, NOT expanded per-arity).
    builtins: Vec<&'static FunctionInfo<'static>>,
    /// Dialect extension functions (owned, copied from C data at construction).
    extensions: Vec<OwnedFunctionInfo>,
    /// User/session-defined functions.
    session: Vec<FunctionDef>,
}

/// Internal owned version for dialect extensions whose lifetime doesn't match `'static`.
#[derive(Clone)]
struct OwnedFunctionInfo {
    name: String,
    arities: Vec<i16>,
    category: FunctionCategory,
}

impl FunctionCatalog {
    /// Build the catalog from a dialect and its compile-time configuration.
    ///
    /// Includes the SQLite built-in catalog and dialect extensions, both
    /// filtered by `config`. Call [`with_session_functions`](Self::with_session_functions)
    /// to merge in user-defined functions.
    pub fn for_dialect(env: &DialectEnv<'_>) -> Self {
        #[cfg(feature = "sqlite")]
        let builtins: Vec<&'static FunctionInfo<'static>> =
            syntaqlite_parser::available_functions(env);

        #[cfg(not(feature = "sqlite"))]
        let builtins: Vec<&'static FunctionInfo<'static>> = Vec::new();

        let extensions: Vec<OwnedFunctionInfo> = env
            .function_extensions()
            .into_iter()
            .filter(|ext| syntaqlite_parser::is_function_available(ext, env))
            .map(|ext| OwnedFunctionInfo {
                name: ext.info.name.to_string(),
                arities: ext.info.arities.to_vec(),
                category: ext.info.category,
            })
            .collect();

        FunctionCatalog {
            builtins,
            extensions,
            session: Vec::new(),
        }
    }

    /// Build the catalog using default configuration. Convenience for SQLite.
    #[cfg(feature = "sqlite")]
    pub fn for_default_dialect(env: &DialectEnv<'_>) -> Self {
        Self::for_dialect(env)
    }

    /// Append user-defined functions from a list of session functions.
    pub fn add_session_functions(&mut self, functions: &[FunctionDef]) {
        self.session.extend(functions.iter().cloned());
    }

    /// Check whether a function call with the given name and argument count is valid.
    pub fn check_call(&self, name: &str, arg_count: usize) -> FunctionCheckResult {
        let mut found = false;
        let mut expected = Vec::new();
        let mut has_variadic = false;

        // Check builtins.
        for info in &self.builtins {
            if info.name.eq_ignore_ascii_case(name) {
                found = true;
                if matches_arity(info.arities, arg_count) {
                    return FunctionCheckResult::Ok;
                }
                collect_arities(info.arities, &mut expected, &mut has_variadic);
            }
        }

        // Check dialect extensions.
        for ext in &self.extensions {
            if ext.name.eq_ignore_ascii_case(name) {
                found = true;
                if matches_arity(&ext.arities, arg_count) {
                    return FunctionCheckResult::Ok;
                }
                collect_arities(&ext.arities, &mut expected, &mut has_variadic);
            }
        }

        // Check session/document-defined functions.
        for func in &self.session {
            if func.name.eq_ignore_ascii_case(name) {
                found = true;
                match func.args {
                    None => return FunctionCheckResult::Ok,
                    Some(n) if n == arg_count => return FunctionCheckResult::Ok,
                    Some(n) => expected.push(n),
                }
            }
        }

        if !found {
            return FunctionCheckResult::Unknown;
        }

        // If any source had a variadic entry, the call is valid for any arity.
        // (We should not reach here if that's the case since matches_arity
        // handles it, but be safe.)
        if has_variadic {
            return FunctionCheckResult::Ok;
        }

        expected.sort_unstable();
        expected.dedup();
        FunctionCheckResult::WrongArity { expected }
    }

    /// Look up a function by name. Returns `None` if not found.
    pub fn lookup(&self, name: &str) -> Option<FunctionLookup<'_>> {
        // Check builtins first.
        for info in &self.builtins {
            if info.name.eq_ignore_ascii_case(name) {
                let mut fixed_arities = Vec::new();
                let mut is_variadic = false;
                collect_arities(info.arities, &mut fixed_arities, &mut is_variadic);
                fixed_arities.sort_unstable();
                fixed_arities.dedup();
                return Some(FunctionLookup {
                    name: info.name,
                    category: info.category,
                    fixed_arities,
                    is_variadic,
                });
            }
        }

        // Check extensions.
        for ext in &self.extensions {
            if ext.name.eq_ignore_ascii_case(name) {
                let mut fixed_arities = Vec::new();
                let mut is_variadic = false;
                collect_arities(&ext.arities, &mut fixed_arities, &mut is_variadic);
                fixed_arities.sort_unstable();
                fixed_arities.dedup();
                return Some(FunctionLookup {
                    name: &ext.name,
                    category: ext.category,
                    fixed_arities,
                    is_variadic,
                });
            }
        }

        // Check session.
        for func in &self.session {
            if func.name.eq_ignore_ascii_case(name) {
                return Some(FunctionLookup {
                    name: &func.name,
                    category: FunctionCategory::Scalar,
                    fixed_arities: func.args.into_iter().collect(),
                    is_variadic: func.args.is_none(),
                });
            }
        }

        None
    }

    /// All unique function names (deduplicated, for completions and fuzzy matching).
    pub fn all_names(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut names = Vec::new();

        for info in &self.builtins {
            if seen.insert(info.name.to_ascii_lowercase()) {
                names.push(info.name.to_string());
            }
        }
        for ext in &self.extensions {
            if seen.insert(ext.name.to_ascii_lowercase()) {
                names.push(ext.name.clone());
            }
        }
        for func in &self.session {
            if seen.insert(func.name.to_ascii_lowercase()) {
                names.push(func.name.clone());
            }
        }

        names
    }

    /// Iterate all known functions as `(name, category)` pairs.
    ///
    /// Each function name appears once even if multiple arities exist.
    pub fn iter(&self) -> impl Iterator<Item = (&str, FunctionCategory)> {
        let mut seen = std::collections::HashSet::new();
        let mut result: Vec<(&str, FunctionCategory)> = Vec::new();

        for info in &self.builtins {
            if seen.insert(info.name.to_ascii_lowercase()) {
                result.push((info.name, info.category));
            }
        }
        for ext in &self.extensions {
            if seen.insert(ext.name.to_ascii_lowercase()) {
                result.push((&ext.name, ext.category));
            }
        }
        for func in &self.session {
            if seen.insert(func.name.to_ascii_lowercase()) {
                result.push((&func.name, FunctionCategory::Scalar));
            }
        }

        result.into_iter()
    }

    /// Unique function names as `&str`, deduplicated across arities.
    pub fn unique_names(&self) -> impl Iterator<Item = &str> {
        let mut seen = std::collections::HashSet::new();
        let mut names: Vec<&str> = Vec::new();

        for info in &self.builtins {
            if seen.insert(info.name.to_ascii_lowercase()) {
                names.push(info.name);
            }
        }
        for ext in &self.extensions {
            if seen.insert(ext.name.to_ascii_lowercase()) {
                names.push(&ext.name);
            }
        }
        for func in &self.session {
            if seen.insert(func.name.to_ascii_lowercase()) {
                names.push(&func.name);
            }
        }

        names.into_iter()
    }
}

/// Check if a given `arg_count` matches any arity in the arities slice.
///
/// Arity encoding:
/// - Positive value: exact arity (e.g., `2` = exactly 2 args)
/// - `-1`: any number of args
/// - `-N` (N > 1): at least `N - 1` args
/// - Empty slice: variadic (any arity)
fn matches_arity(arities: &[i16], arg_count: usize) -> bool {
    if arities.is_empty() {
        return true;
    }
    arities.iter().any(|&a| {
        if a < 0 {
            if a == -1 {
                true
            } else {
                arg_count >= (-a - 1) as usize
            }
        } else {
            arg_count == a as usize
        }
    })
}

/// Collect fixed arities and set `is_variadic` if any entry is variadic.
fn collect_arities(arities: &[i16], expected: &mut Vec<usize>, is_variadic: &mut bool) {
    if arities.is_empty() {
        *is_variadic = true;
        return;
    }
    for &a in arities {
        if a < 0 {
            *is_variadic = true;
        } else {
            expected.push(a as usize);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_arity_exact() {
        assert!(matches_arity(&[2], 2));
        assert!(!matches_arity(&[2], 3));
    }

    #[test]
    fn matches_arity_variadic_any() {
        assert!(matches_arity(&[-1], 0));
        assert!(matches_arity(&[-1], 100));
    }

    #[test]
    fn matches_arity_variadic_min() {
        // -4 means at least 3 args
        assert!(!matches_arity(&[-4], 2));
        assert!(matches_arity(&[-4], 3));
        assert!(matches_arity(&[-4], 10));
    }

    #[test]
    fn matches_arity_empty_is_variadic() {
        assert!(matches_arity(&[], 0));
        assert!(matches_arity(&[], 42));
    }

    #[test]
    fn matches_arity_mixed() {
        // 1 or 2 or variadic(any)
        assert!(matches_arity(&[1, 2, -1], 0));
        assert!(matches_arity(&[1, 2, -1], 5));
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn catalog_check_call_abs() {
        let dialect = syntaqlite_parser_sqlite::dialect();
        let catalog = FunctionCatalog::for_default_dialect(&dialect);
        assert!(matches!(
            catalog.check_call("abs", 1),
            FunctionCheckResult::Ok
        ));
        assert!(matches!(
            catalog.check_call("abs", 2),
            FunctionCheckResult::WrongArity { .. }
        ));
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn catalog_check_call_unknown() {
        let dialect = syntaqlite_parser_sqlite::dialect();
        let catalog = FunctionCatalog::for_default_dialect(&dialect);
        assert!(matches!(
            catalog.check_call("no_such_func", 1),
            FunctionCheckResult::Unknown
        ));
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn catalog_check_call_session_function() {
        let dialect = syntaqlite_parser_sqlite::dialect();
        let mut catalog = FunctionCatalog::for_default_dialect(&dialect);
        catalog.add_session_functions(&[FunctionDef {
            name: "my_func".to_string(),
            args: Some(2),
        }]);
        assert!(matches!(
            catalog.check_call("my_func", 2),
            FunctionCheckResult::Ok
        ));
        assert!(matches!(
            catalog.check_call("my_func", 3),
            FunctionCheckResult::WrongArity { .. }
        ));
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn catalog_lookup_abs() {
        let dialect = syntaqlite_parser_sqlite::dialect();
        let catalog = FunctionCatalog::for_default_dialect(&dialect);
        let info = catalog.lookup("abs").expect("abs should exist");
        assert_eq!(info.name, "abs");
        assert!(!info.is_variadic);
        assert!(info.fixed_arities.contains(&1));
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn catalog_all_names_includes_builtins() {
        let dialect = syntaqlite_parser_sqlite::dialect();
        let catalog = FunctionCatalog::for_default_dialect(&dialect);
        let names = catalog.all_names();
        assert!(names.iter().any(|n| n == "abs"));
        assert!(names.iter().any(|n| n == "count"));
    }
}

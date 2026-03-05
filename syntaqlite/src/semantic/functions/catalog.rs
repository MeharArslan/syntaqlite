// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use crate::dialect::catalog::{FunctionCategory, FunctionInfo, is_function_available};
use crate::dialect::Dialect;

use super::types::{FunctionCheckResult, FunctionDef, FunctionLookup};

/// Resolved function catalog for a dialect + config combination.
///
/// Merges three sources with the following priority:
/// 1. `SQLite` built-in catalog (filtered by [`Dialect`])
/// 2. Dialect extension functions (filtered by [`Dialect`])
/// 3. Session/document user-defined functions
///
/// Unlike the old `Vec<FunctionDef>` approach, this does **not** expand
/// one entry per arity - arity checking works directly on the compact
/// `&[i16]` representation from `FunctionInfo`.
#[derive(Clone)]
pub(crate) struct FunctionCatalog {
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
    pub(crate) fn for_dialect(dialect: &Dialect) -> Self {
        #[cfg(feature = "sqlite")]
        let builtins: Vec<&'static FunctionInfo<'static>> =
            crate::sqlite::functions_catalog::SQLITE_FUNCTIONS
                .iter()
                .filter(|e| is_function_available(e, dialect))
                .map(|e| &e.info)
                .collect();

        #[cfg(not(feature = "sqlite"))]
        let builtins: Vec<&'static FunctionInfo<'static>> = Vec::new();

        let extensions: Vec<OwnedFunctionInfo> = dialect
            .function_extensions()
            .iter()
            .filter(|ext| is_function_available(ext, dialect))
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

    /// Append user-defined functions from a list of session functions.
    pub(crate) fn add_session_functions(&mut self, functions: &[FunctionDef]) {
        self.session.extend(functions.iter().cloned());
    }

    /// Check whether a function call with the given name and argument count is valid.
    pub(crate) fn check_call(&self, name: &str, arg_count: usize) -> FunctionCheckResult {
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

        if has_variadic {
            return FunctionCheckResult::Ok;
        }

        expected.sort_unstable();
        expected.dedup();
        FunctionCheckResult::WrongArity { expected }
    }

    /// Look up a function by name. Returns `None` if not found.
    pub(crate) fn lookup(&self, name: &str) -> Option<FunctionLookup<'_>> {
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
    pub(crate) fn all_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        self.visit_unique_names(|name, _category| names.push(name.to_string()));
        names
    }

    /// Iterate all known functions as `(name, category)` pairs.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, FunctionCategory)> {
        let mut result = Vec::new();
        self.visit_unique_names(|name, category| result.push((name, category)));
        result.into_iter()
    }

    /// Unique function names as `&str`, deduplicated across arities.
    pub(crate) fn unique_names(&self) -> impl Iterator<Item = &str> {
        let mut names = Vec::new();
        self.visit_unique_names(|name, _category| names.push(name));
        names.into_iter()
    }

    fn visit_unique_names<'a>(&'a self, mut visit: impl FnMut(&'a str, FunctionCategory)) {
        let mut seen = std::collections::HashSet::new();

        for info in &self.builtins {
            if seen.insert(info.name.to_ascii_lowercase()) {
                visit(info.name, info.category);
            }
        }
        for ext in &self.extensions {
            if seen.insert(ext.name.to_ascii_lowercase()) {
                visit(&ext.name, ext.category);
            }
        }
        for func in &self.session {
            if seen.insert(func.name.to_ascii_lowercase()) {
                visit(&func.name, FunctionCategory::Scalar);
            }
        }
    }
}

fn matches_arity(arities: &[i16], arg_count: usize) -> bool {
    if arities.is_empty() {
        return true;
    }
    arities.iter().any(|&a| {
        if a < 0 {
            if a == -1 {
                true
            } else {
                arg_count
                    >= usize::try_from(-i32::from(a) - 1)
                        .expect("negative arity encodes minimum args")
            }
        } else {
            arg_count == usize::try_from(i32::from(a)).expect("fixed arity is non-negative")
        }
    })
}

fn collect_arities(arities: &[i16], expected: &mut Vec<usize>, is_variadic: &mut bool) {
    if arities.is_empty() {
        *is_variadic = true;
        return;
    }
    for &a in arities {
        if a < 0 {
            *is_variadic = true;
        } else {
            expected.push(usize::try_from(i32::from(a)).expect("fixed arity is non-negative"));
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
        assert!(matches_arity(&[1, 2, -1], 0));
        assert!(matches_arity(&[1, 2, -1], 5));
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn catalog_check_call_abs() {
        let dialect = crate::dialect::sqlite();
        let catalog = FunctionCatalog::for_dialect(&dialect);
        assert!(matches!(
            catalog.check_call("abs", 1),
            FunctionCheckResult::Ok
        ));
        assert!(matches!(
            catalog.check_call("abs", 2),
            FunctionCheckResult::WrongArity { .. }
        ));
    }
}

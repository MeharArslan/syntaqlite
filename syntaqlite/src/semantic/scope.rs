// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashMap;

use super::catalog::CatalogStack;

/// A single scope level (e.g., one SELECT or subquery).
#[derive(Debug, Default)]
struct Scope {
    /// Lowercase table/alias name → optional column names.
    /// `None` means the table exists but columns are unknown.
    tables: HashMap<String, Option<Vec<String>>>,
}

/// A stack of scopes for name resolution, with ambient catalog.
///
/// Resolution order for names: SQL scope stack → document → database → static.
pub(crate) struct ScopeStack<'ctx> {
    catalog: CatalogStack<'ctx>,
    stack: Vec<Scope>,
}

impl<'ctx> ScopeStack<'ctx> {
    pub(crate) fn new(catalog: CatalogStack<'ctx>) -> Self {
        ScopeStack {
            catalog,
            stack: vec![Scope::default()],
        }
    }

    pub(crate) fn push(&mut self) {
        self.stack.push(Scope::default());
    }

    pub(crate) fn pop(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
    }

    /// Add a table or alias to the current scope.
    /// `columns` is `None` if column info is not available.
    pub(crate) fn add_table(&mut self, name: &str, columns: Option<Vec<String>>) {
        self.stack
            .last_mut()
            .unwrap()
            .tables
            .insert(name.to_ascii_lowercase(), columns);
    }

    /// Look up column names for a table from the ambient catalog.
    /// Returns `Some(columns)` if the table exists (may be empty if no columns
    /// could be inferred), `None` if the table is not found.
    pub(crate) fn ambient_columns_for_table(&self, name: &str) -> Option<Vec<String>> {
        self.catalog.columns_for(name)
    }

    pub(crate) fn resolve_table(&self, name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        self.stack.iter().any(|s| s.tables.contains_key(&lower))
            || self.catalog.resolve_relation(name)
    }

    /// Resolve a column reference.
    ///
    /// Qualified (`table.column`): look up the specific table's columns.
    /// Unqualified (`column`): search all tables in scope + ambient.
    pub(crate) fn resolve_column(&self, table: Option<&str>, column: &str) -> ColumnResolution {
        if let Some(tbl) = table {
            return self.resolve_qualified_column(tbl, column);
        }

        let mut has_unknown = false;
        for scope in self.stack.iter().rev() {
            for cols in scope.tables.values() {
                match cols {
                    Some(col_list) => {
                        if col_list.iter().any(|c| c.eq_ignore_ascii_case(column)) {
                            return ColumnResolution::Found;
                        }
                    }
                    // A table with unknown columns — can't reject, but keep looking.
                    None => has_unknown = true,
                }
            }
        }

        // Check ambient catalog columns.
        let ambient_found = self
            .catalog
            .all_column_names(None)
            .iter()
            .any(|c| c.eq_ignore_ascii_case(column));
        if ambient_found {
            return ColumnResolution::Found;
        }

        // If any table in scope has unknown columns, we can't be sure
        // the column doesn't exist — conservatively accept.
        if has_unknown {
            return ColumnResolution::Found;
        }

        ColumnResolution::NotFound
    }

    pub(crate) fn all_table_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .stack
            .iter()
            .flat_map(|s| s.tables.keys().cloned())
            .chain(self.catalog.all_relation_names())
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    /// Collect all column names visible in scope (for fuzzy matching).
    /// If `table` is given, only return columns from that table.
    pub(crate) fn all_column_names(&self, table: Option<&str>) -> Vec<String> {
        let mut names = Vec::new();

        for scope in &self.stack {
            for (key, cols) in &scope.tables {
                if table.is_none_or(|tbl| key.eq_ignore_ascii_case(tbl))
                    && let Some(cols) = cols
                {
                    names.extend(cols.iter().map(|c| c.to_ascii_lowercase()));
                }
            }
        }

        names.extend(self.catalog.all_column_names(table));

        names.sort_unstable();
        names.dedup();
        names
    }

    fn resolve_qualified_column(&self, table: &str, column: &str) -> ColumnResolution {
        let lower = table.to_ascii_lowercase();
        for scope in self.stack.iter().rev() {
            if let Some(cols) = scope.tables.get(&lower) {
                return match cols {
                    Some(col_list) if col_list.iter().any(|c| c.eq_ignore_ascii_case(column)) => {
                        ColumnResolution::Found
                    }
                    Some(_) => ColumnResolution::TableFoundColumnMissing,
                    None => ColumnResolution::Found,
                };
            }
        }
        ColumnResolution::TableNotFound
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ColumnResolution {
    Found,
    TableFoundColumnMissing,
    TableNotFound,
    NotFound,
}

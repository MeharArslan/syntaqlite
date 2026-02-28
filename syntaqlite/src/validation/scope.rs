// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashMap;

use super::types::{DocumentContext, RelationDef, SessionContext};

/// A single scope level (e.g., one SELECT or subquery).
#[derive(Debug, Default)]
struct Scope {
    /// Lowercase table/alias name → optional column names.
    /// `None` means the table exists but columns are unknown.
    tables: HashMap<String, Option<Vec<String>>>,
}

/// A stack of scopes for name resolution, with optional ambient schema.
///
/// Resolution order for names: SQL scope stack → document → session.
pub(super) struct ScopeStack<'ctx> {
    session: Option<&'ctx SessionContext>,
    document: Option<&'ctx DocumentContext>,
    stack: Vec<Scope>,
}

impl<'ctx> ScopeStack<'ctx> {
    pub(super) fn new(
        session: Option<&'ctx SessionContext>,
        document: Option<&'ctx DocumentContext>,
    ) -> Self {
        ScopeStack {
            session,
            document,
            stack: vec![Scope::default()],
        }
    }

    pub(super) fn push(&mut self) {
        self.stack.push(Scope::default());
    }

    pub(super) fn pop(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
    }

    /// Add a table or alias to the current scope.
    /// `columns` is `None` if column info is not available.
    pub(super) fn add_table(&mut self, name: &str, columns: Option<Vec<String>>) {
        self.stack
            .last_mut()
            .unwrap()
            .tables
            .insert(name.to_lowercase(), columns);
    }

    /// Iterate all relations from document context then session context.
    fn ambient_relations(&self) -> impl Iterator<Item = &RelationDef> + '_ {
        self.document
            .into_iter()
            .flat_map(|d| d.relations.iter())
            .chain(self.session.into_iter().flat_map(|s| s.relations.iter()))
    }

    /// Look up column names for a table from the ambient schema context.
    /// Returns `Some(columns)` if the table exists and has columns defined,
    /// `None` if the table is not found or has no columns.
    /// Searches document context first, then session context.
    pub(super) fn ambient_columns_for_table(&self, name: &str) -> Option<Vec<String>> {
        self.ambient_relations()
            .find(|r| r.name.eq_ignore_ascii_case(name) && !r.columns.is_empty())
            .map(|r| r.columns.iter().map(|c| c.name.clone()).collect())
    }

    pub(super) fn resolve_table(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        self.stack.iter().any(|s| s.tables.contains_key(&lower))
            || self.ambient_relations().any(|r| r.name.eq_ignore_ascii_case(name))
    }

    /// Resolve a column reference.
    ///
    /// Qualified (`table.column`): look up the specific table's columns.
    /// Unqualified (`column`): search all tables in scope + ambient.
    pub(super) fn resolve_column(&self, table: Option<&str>, column: &str) -> ColumnResolution {
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

        let ambient_found = self
            .ambient_relations()
            .any(|r| r.columns.iter().any(|c| c.name.eq_ignore_ascii_case(column)));
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

    pub(super) fn all_table_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .stack
            .iter()
            .flat_map(|s| s.tables.keys().cloned())
            .chain(self.ambient_relations().map(|r| r.name.to_lowercase()))
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// Collect all column names visible in scope (for fuzzy matching).
    /// If `table` is given, only return columns from that table.
    pub(super) fn all_column_names(&self, table: Option<&str>) -> Vec<String> {
        let mut names = Vec::new();

        for scope in &self.stack {
            for (key, cols) in &scope.tables {
                if table.is_none_or(|tbl| key.eq_ignore_ascii_case(tbl)) {
                    if let Some(cols) = cols {
                        names.extend(cols.iter().map(|c| c.to_lowercase()));
                    }
                }
            }
        }

        for r in self.ambient_relations() {
            if table.is_none_or(|tbl| r.name.eq_ignore_ascii_case(tbl)) {
                names.extend(r.columns.iter().map(|c| c.name.to_lowercase()));
            }
        }

        names.sort();
        names.dedup();
        names
    }

    fn resolve_qualified_column(&self, table: &str, column: &str) -> ColumnResolution {
        let lower = table.to_lowercase();
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
pub(super) enum ColumnResolution {
    Found,
    TableFoundColumnMissing,
    TableNotFound,
    NotFound,
}

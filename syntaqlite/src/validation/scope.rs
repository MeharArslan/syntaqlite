// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashMap;

use super::types::{DocumentContext, SessionContext};

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

    /// Look up column names for a table from the ambient schema context.
    /// Returns `Some(columns)` if the table exists and has columns defined,
    /// `None` if the table is not found or has no columns.
    /// Searches document context first, then session context.
    pub(super) fn ambient_columns_for_table(&self, name: &str) -> Option<Vec<String>> {
        if let Some(doc) = self.document {
            for t in &doc.tables {
                if t.name.eq_ignore_ascii_case(name) && !t.columns.is_empty() {
                    return Some(t.columns.iter().map(|c| c.name.clone()).collect());
                }
            }
            for v in &doc.views {
                if v.name.eq_ignore_ascii_case(name) && !v.columns.is_empty() {
                    return Some(v.columns.iter().map(|c| c.name.clone()).collect());
                }
            }
        }
        let ctx = self.session?;
        for t in &ctx.tables {
            if t.name.eq_ignore_ascii_case(name) && !t.columns.is_empty() {
                return Some(t.columns.iter().map(|c| c.name.clone()).collect());
            }
        }
        for v in &ctx.views {
            if v.name.eq_ignore_ascii_case(name) && !v.columns.is_empty() {
                return Some(v.columns.iter().map(|c| c.name.clone()).collect());
            }
        }
        None
    }

    pub(super) fn resolve_table(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        self.stack.iter().any(|s| s.tables.contains_key(&lower)) || self.ambient_has_table(name)
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

        if self.ambient_has_column(column) {
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
            .collect();
        if let Some(doc) = self.document {
            names.extend(doc.tables.iter().map(|t| t.name.to_lowercase()));
            names.extend(doc.views.iter().map(|v| v.name.to_lowercase()));
        }
        if let Some(ctx) = self.session {
            names.extend(ctx.tables.iter().map(|t| t.name.to_lowercase()));
            names.extend(ctx.views.iter().map(|v| v.name.to_lowercase()));
        }
        names.sort();
        names.dedup();
        names
    }

    /// Collect all column names visible in scope (for fuzzy matching).
    /// If `table` is given, only return columns from that table.
    pub(super) fn all_column_names(&self, table: Option<&str>) -> Vec<String> {
        let mut names = Vec::new();

        match table {
            Some(tbl) => {
                let lower = tbl.to_lowercase();
                for scope in &self.stack {
                    if let Some(Some(cols)) = scope.tables.get(&lower) {
                        names.extend(cols.iter().map(|c| c.to_lowercase()));
                    }
                }
                if let Some(doc) = self.document {
                    for t in &doc.tables {
                        if t.name.eq_ignore_ascii_case(tbl) {
                            names.extend(t.columns.iter().map(|c| c.name.to_lowercase()));
                        }
                    }
                    for v in &doc.views {
                        if v.name.eq_ignore_ascii_case(tbl) {
                            names.extend(v.columns.iter().map(|c| c.name.to_lowercase()));
                        }
                    }
                }
                if let Some(ctx) = self.session {
                    for t in &ctx.tables {
                        if t.name.eq_ignore_ascii_case(tbl) {
                            names.extend(t.columns.iter().map(|c| c.name.to_lowercase()));
                        }
                    }
                    for v in &ctx.views {
                        if v.name.eq_ignore_ascii_case(tbl) {
                            names.extend(v.columns.iter().map(|c| c.name.to_lowercase()));
                        }
                    }
                }
            }
            None => {
                for scope in &self.stack {
                    for cols in scope.tables.values().flatten() {
                        names.extend(cols.iter().map(|c| c.to_lowercase()));
                    }
                }
                if let Some(doc) = self.document {
                    for t in &doc.tables {
                        names.extend(t.columns.iter().map(|c| c.name.to_lowercase()));
                    }
                    for v in &doc.views {
                        names.extend(v.columns.iter().map(|c| c.name.to_lowercase()));
                    }
                }
                if let Some(ctx) = self.session {
                    for t in &ctx.tables {
                        names.extend(t.columns.iter().map(|c| c.name.to_lowercase()));
                    }
                    for v in &ctx.views {
                        names.extend(v.columns.iter().map(|c| c.name.to_lowercase()));
                    }
                }
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

    fn ambient_has_table(&self, name: &str) -> bool {
        if let Some(doc) = self.document {
            if doc.tables.iter().any(|t| t.name.eq_ignore_ascii_case(name))
                || doc.views.iter().any(|v| v.name.eq_ignore_ascii_case(name))
            {
                return true;
            }
        }
        let Some(ctx) = self.session else {
            return false;
        };
        ctx.tables.iter().any(|t| t.name.eq_ignore_ascii_case(name))
            || ctx.views.iter().any(|v| v.name.eq_ignore_ascii_case(name))
    }

    fn ambient_has_column(&self, column: &str) -> bool {
        if let Some(doc) = self.document {
            let found = doc.tables.iter().any(|t| {
                t.columns.iter().any(|c| c.name.eq_ignore_ascii_case(column))
            }) || doc.views.iter().any(|v| {
                v.columns.iter().any(|c| c.name.eq_ignore_ascii_case(column))
            });
            if found {
                return true;
            }
        }
        let Some(ctx) = self.session else {
            return false;
        };
        ctx.tables.iter().any(|t| {
            t.columns
                .iter()
                .any(|c| c.name.eq_ignore_ascii_case(column))
        }) || ctx.views.iter().any(|v| {
            v.columns
                .iter()
                .any(|c| c.name.eq_ignore_ascii_case(column))
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ColumnResolution {
    Found,
    TableFoundColumnMissing,
    TableNotFound,
    NotFound,
}

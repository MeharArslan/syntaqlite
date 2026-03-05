// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use super::types::{ColumnDef, RelationDef};

/// A non-owning view over session and document relation slices.
///
/// Cheap to construct per-statement (two slice refs). Document relations
/// take priority over session relations for name resolution.
#[derive(Clone, Copy)]
pub(crate) struct RelationCatalog<'a> {
    pub(crate) session: &'a [RelationDef],
    pub(crate) document: &'a [RelationDef],
}

impl<'a> RelationCatalog<'a> {
    /// Construct from two slices. Document relations take priority.
    pub(crate) fn new(session: &'a [RelationDef], document: &'a [RelationDef]) -> Self {
        RelationCatalog { session, document }
    }

    /// Case-insensitive lookup, document-first priority.
    pub(crate) fn lookup(&self, name: &str) -> Option<&'a RelationDef> {
        self.document
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(name))
            .or_else(|| {
                self.session
                    .iter()
                    .find(|r| r.name.eq_ignore_ascii_case(name))
            })
    }

    /// Existence check (case-insensitive).
    pub(crate) fn resolve(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }

    /// Column lookup for a relation by name.
    pub(crate) fn columns_for(&self, name: &str) -> Option<&'a [ColumnDef]> {
        self.lookup(name).map(|r| r.columns.as_slice())
    }

    /// All relation names, deduplicated (for fuzzy matching).
    pub(crate) fn all_names(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut names = Vec::new();
        for r in self.iter() {
            let lower = r.name.to_ascii_lowercase();
            if seen.insert(lower.clone()) {
                names.push(lower);
            }
        }
        names
    }

    /// All column names, optionally filtered by table name (for fuzzy matching).
    pub(crate) fn all_column_names(&self, table: Option<&str>) -> Vec<String> {
        let mut names = Vec::new();
        for r in self.iter() {
            if table.is_none_or(|tbl| r.name.eq_ignore_ascii_case(tbl)) {
                names.extend(r.columns.iter().map(|c| c.name.to_ascii_lowercase()));
            }
        }
        names.sort_unstable();
        names.dedup();
        names
    }

    /// Iterate all relations (document first, then session).
    pub(crate) fn iter(&self) -> impl Iterator<Item = &'a RelationDef> {
        self.document.iter().chain(self.session.iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::relations::{ColumnDef, RelationDef, RelationKind};

    fn col(name: &str) -> ColumnDef {
        ColumnDef {
            name: name.to_string(),
            type_name: None,
            is_primary_key: false,
            is_nullable: true,
        }
    }

    fn table(name: &str, cols: &[&str]) -> RelationDef {
        RelationDef {
            name: name.to_string(),
            columns: cols.iter().map(|c| col(c)).collect(),
            kind: RelationKind::Table,
        }
    }

    #[test]
    fn lookup_document_first() {
        let session = [table("users", &["id", "email"])];
        let document = [table("users", &["id", "name"])];
        let catalog = RelationCatalog::new(&session, &document);
        let r = catalog.lookup("users").unwrap();
        assert_eq!(r.columns[1].name, "name"); // document wins
    }

    #[test]
    fn lookup_falls_back_to_session() {
        let session = [table("orders", &["id"])];
        let document = [];
        let catalog = RelationCatalog::new(&session, &document);
        assert!(catalog.lookup("orders").is_some());
    }

    #[test]
    fn lookup_case_insensitive() {
        let session = [table("Users", &["id"])];
        let catalog = RelationCatalog::new(&session, &[]);
        assert!(catalog.lookup("USERS").is_some());
        assert!(catalog.lookup("users").is_some());
    }

    #[test]
    fn resolve_returns_false_for_unknown() {
        let catalog = RelationCatalog::new(&[], &[]);
        assert!(!catalog.resolve("nope"));
    }

    #[test]
    fn columns_for_returns_columns() {
        let session = [table("t", &["a", "b"])];
        let catalog = RelationCatalog::new(&session, &[]);
        let cols = catalog.columns_for("t").unwrap();
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0].name, "a");
    }

    #[test]
    fn all_names_deduplicates() {
        let session = [table("T", &[])];
        let document = [table("t", &[])];
        let catalog = RelationCatalog::new(&session, &document);
        let names = catalog.all_names();
        assert_eq!(names.len(), 1);
    }

    #[test]
    fn all_column_names_filters_by_table() {
        let session = [table("a", &["x", "y"]), table("b", &["z"])];
        let catalog = RelationCatalog::new(&session, &[]);
        let cols = catalog.all_column_names(Some("a"));
        assert_eq!(cols, vec!["x", "y"]);
    }

    #[test]
    fn all_column_names_all_tables() {
        let session = [table("a", &["x"]), table("b", &["y"])];
        let catalog = RelationCatalog::new(&session, &[]);
        let cols = catalog.all_column_names(None);
        assert_eq!(cols, vec!["x", "y"]);
    }

    #[test]
    fn iter_yields_document_then_session() {
        let session = [table("s", &[])];
        let document = [table("d", &[])];
        let catalog = RelationCatalog::new(&session, &document);
        let names: Vec<_> = catalog.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["d", "s"]);
    }
}

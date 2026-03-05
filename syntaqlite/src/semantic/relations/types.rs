// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

/// Whether a [`RelationDef`] represents a base table or a view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    /// Base table relation.
    Table,
    /// View relation.
    View,
}

/// A table or view in the schema.
#[derive(Clone)]
pub struct RelationDef {
    /// Relation name.
    pub name: String,
    /// Projected column definitions for the relation.
    pub columns: Vec<ColumnDef>,
    /// Relation kind (table or view).
    pub kind: RelationKind,
}

/// Column definition used by semantic relation catalogs.
#[derive(Clone)]
pub struct ColumnDef {
    /// Column name.
    pub name: String,
    /// Optional declared type name. `SQLite` is permissive here.
    pub type_name: Option<String>,
    /// Whether the column participates in a primary key.
    pub is_primary_key: bool,
    /// Whether the column accepts NULL values.
    pub is_nullable: bool,
}

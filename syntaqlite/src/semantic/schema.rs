// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Shared schema entities for validation.

/// A user-defined or database function (from DDL parsing, JSON config, or runtime).
#[derive(Debug, Clone)]
pub(crate) struct FunctionDef {
    /// Function name.
    pub name: String,
    /// `None` means variadic (any number of arguments).
    pub args: Option<usize>,
}

/// Backward-compatible alias.
pub(crate) type SessionFunction = FunctionDef;

/// Whether a [`RelationDef`] represents a base table or a view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelationKind {
    /// Base table relation.
    Table,
    /// View relation.
    View,
}

/// A table or view in the schema.
#[derive(Clone)]
pub(crate) struct RelationDef {
    /// Relation name.
    pub name: String,
    /// Projected column definitions for the relation.
    pub columns: Vec<ColumnDef>,
    /// Relation kind (table or view).
    pub kind: RelationKind,
}

/// Column definition used by semantic relation catalogs.
#[derive(Clone)]
pub(crate) struct ColumnDef {
    /// Column name.
    pub name: String,
    /// Optional declared type name. `SQLite` is permissive here.
    pub type_name: Option<String>,
    /// Whether the column participates in a primary key.
    pub is_primary_key: bool,
    /// Whether the column accepts NULL values.
    pub is_nullable: bool,
}

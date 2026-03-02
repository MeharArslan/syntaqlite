// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

/// Whether a [`RelationDef`] represents a base table or a view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    Table,
    View,
}

/// A table or view in the schema.
#[derive(Clone)]
pub struct RelationDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub kind: RelationKind,
}

#[derive(Clone)]
pub struct ColumnDef {
    pub name: String,
    /// SQLite is flexible with types.
    pub type_name: Option<String>,
    pub is_primary_key: bool,
    pub is_nullable: bool,
}

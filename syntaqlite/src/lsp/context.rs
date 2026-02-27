// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

/// Database schema context for analysis.
///
/// Not used in Phase 1 (parse-level diagnostics only), but the API
/// is established now so future phases can use it for completions,
/// hover, column validation, etc.
///
/// Callers populate it however they want: introspecting a live DB,
/// parsing CREATE statements, loading from a config file, etc.
pub struct AmbientContext {
    pub tables: Vec<TableDef>,
    pub views: Vec<ViewDef>,
    pub functions: Vec<FunctionDef>,
}

pub struct TableDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
}

pub struct ColumnDef {
    pub name: String,
    /// SQLite is flexible with types.
    pub type_name: Option<String>,
    pub is_primary_key: bool,
    pub is_nullable: bool,
}

pub struct ViewDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
}

pub struct FunctionDef {
    pub name: String,
    /// None = variadic.
    pub args: Option<usize>,
    pub description: Option<String>,
}

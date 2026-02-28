// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

/// A diagnostic message associated with a source range.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Byte offset of the start of the diagnostic range.
    pub start_offset: usize,
    /// Byte offset of the end of the diagnostic range.
    pub end_offset: usize,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Severity level.
    pub severity: Severity,
}

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Database schema context for analysis.
///
/// Callers populate it however they want: introspecting a live DB,
/// parsing CREATE statements, loading from a config file, etc.
pub struct SessionContext {
    pub tables: Vec<TableDef>,
    pub views: Vec<ViewDef>,
    pub functions: Vec<FunctionDef>,
}

/// Deprecated: renamed to [`SessionContext`].
pub type AmbientContext = SessionContext;

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

#[derive(Clone)]
pub struct FunctionDef {
    pub name: String,
    /// None = variadic.
    pub args: Option<usize>,
    pub description: Option<String>,
}

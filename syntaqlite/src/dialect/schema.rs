// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Schema contribution types for dialect-defined schema objects.

/// What kind of schema object a node contributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaKind {
    Table,
    View,
    Function,
    Import,
}

/// A schema contribution: describes how a specific AST node contributes to
/// a schema object (table, view, function, or import).
#[derive(Debug, Clone, Copy)]
pub struct SchemaContribution {
    /// The AST node tag this contribution applies to.
    pub node_tag: u32,
    pub kind: SchemaKind,
    pub name_field: u8,
    pub columns_field: Option<u8>,
    pub select_field: Option<u8>,
    pub args_field: Option<u8>,
}

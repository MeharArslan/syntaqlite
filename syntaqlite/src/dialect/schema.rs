// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Schema contribution types for dialect-defined schema objects.

/// What kind of schema object a node contributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchemaKind {
    Table,
    View,
    Function,
    Import,
}

/// A schema contribution: describes how a specific AST node contributes to
/// a schema object (table, view, function, or import).
#[derive(Debug, Clone, Copy)]
pub(crate) struct SchemaContribution {
    /// The AST node tag this contribution applies to.
    pub node_tag: u32,
    pub kind: SchemaKind,
    pub name_field: u8,
    pub columns_field: Option<u8>,
    pub select_field: Option<u8>,
    pub args_field: Option<u8>,
}

/// Index into a node's field array (0-based).
pub(crate) type FieldIdx = u8;

/// The semantic role assigned to an AST node type.
///
/// Generated from `semantic { ... }` annotations in `.synq` files and stored
/// in a static array indexed by node tag. `Transparent` means the engine
/// recurses into children without special handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SemanticRole {
    // ── Catalog roles (replaces SchemaContribution) ───────────────────────
    DefineTable {
        name: FieldIdx,
        columns: Option<FieldIdx>,
        select: Option<FieldIdx>,
    },
    DefineView {
        name: FieldIdx,
        select: FieldIdx,
    },
    DefineFunction {
        name: FieldIdx,
        args: Option<FieldIdx>,
    },
    Import {
        module: FieldIdx,
    },
    /// No semantic role — recurse into children generically.
    Transparent,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_role_variants_exist() {
        let _ = SemanticRole::Transparent;
        let _ = SemanticRole::DefineTable { name: 0, columns: None, select: None };
        let _ = SemanticRole::DefineView { name: 0, select: 1 };
        let _ = SemanticRole::DefineFunction { name: 0, args: None };
        let _ = SemanticRole::Import { module: 0 };
    }

    #[test]
    fn field_idx_is_u8() {
        let _: FieldIdx = 42u8;
    }
}

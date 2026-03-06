// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Semantic role types for dialect-defined AST annotations.

/// Index into a node's field array (0-based).
pub(crate) type FieldIdx = u8;

/// The kind of relation a `SourceRef` binding introduces into scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelationKind {
    /// Standard SQL table or view.
    Table,
    /// View — kept separate for catalog queries.
    View,
    /// Perfetto interval-structured data.
    Interval,
    /// Perfetto tree-structured data.
    Tree,
    /// Perfetto graph-structured data.
    Graph,
}

/// The semantic role assigned to an AST node type.
///
/// Generated from `semantic { ... }` annotations in `.synq` files and stored
/// in a static array indexed by node tag. `Transparent` means the engine
/// recurses into children without special handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SemanticRole {
    // ── Catalog roles ─────────────────────────────────────────────────────
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

    // ── Column-list items — used during define_table column extraction ─────
    ColumnDef {
        name: FieldIdx,
        type_: Option<FieldIdx>,
        constraints: Option<FieldIdx>,
    },

    // ── Result columns — used during SELECT column inference ───────────────
    ResultColumn {
        flags: FieldIdx,
        alias: FieldIdx,
        expr: FieldIdx,
    },

    // ── Expressions ───────────────────────────────────────────────────────
    /// Function/aggregate/window call: validate name and arg count.
    Call {
        name: FieldIdx,
        args: FieldIdx,
    },
    /// Column reference: validate column and optional table qualifier.
    ColumnRef {
        column: FieldIdx,
        table: FieldIdx,
    },

    // ── Sources ───────────────────────────────────────────────────────────
    /// Table/view reference in FROM — adds binding to current scope.
    SourceRef {
        kind: RelationKind,
        name: FieldIdx,
        alias: FieldIdx,
    },
    /// Subquery in FROM — opens a fresh scope, then binds alias in outer scope.
    ScopedSource {
        body: FieldIdx,
        alias: FieldIdx,
    },

    // ── Scope structure ───────────────────────────────────────────────────
    /// SELECT statement: process `from` first, then validate `exprs`.
    Query {
        from: FieldIdx,
        columns: FieldIdx,
        where_clause: FieldIdx,
        groupby: FieldIdx,
        having: FieldIdx,
        orderby: FieldIdx,
        limit_clause: FieldIdx,
    },
    /// CTE definition: binds a name to a subquery body.
    CteBinding {
        name: FieldIdx,
        body: FieldIdx,
    },
    /// WITH clause: sequential CTE scope wrapping a main query.
    CteScope {
        recursive: FieldIdx,
        bindings: FieldIdx,
        body: FieldIdx,
    },
    /// CREATE TRIGGER: injects OLD/NEW into the trigger body scope.
    TriggerScope {
        target: FieldIdx,
        when: FieldIdx,
        body: FieldIdx,
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
        let _ = SemanticRole::DefineTable {
            name: 0,
            columns: None,
            select: None,
        };
        let _ = SemanticRole::DefineView { name: 0, select: 1 };
        let _ = SemanticRole::DefineFunction {
            name: 0,
            args: None,
        };
        let _ = SemanticRole::Import { module: 0 };
    }

    #[test]
    fn field_idx_is_u8() {
        let _: FieldIdx = 42u8;
    }
}

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Dialect handle, semantic role types, and function catalog types.

use std::mem::size_of;

use syntaqlite_syntax::any::AnyGrammar;
use syntaqlite_syntax::util::SqliteVersion;

// ── Semantic role types ───────────────────────────────────────────────────────

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
        columns: Option<FieldIdx>,
        select: FieldIdx,
    },
    DefineFunction {
        name: FieldIdx,
        args: Option<FieldIdx>,
        /// Optional field index of a return-type child node.
        /// The accumulator looks up that child's [`SemanticRole`] and dispatches
        /// on [`SemanticRole::ReturnSpec`] to determine if the function is
        /// table-returning.
        return_type: Option<FieldIdx>,
    },
    /// Annotates a return-type descriptor node (e.g. `PerfettoReturnType`).
    ///
    /// `columns` points to a column-list child; a non-null NodeId at runtime
    /// means the enclosing function is table-returning. Any dialect with a
    /// dedicated return-type descriptor node can use this role regardless of
    /// how the surrounding syntax is structured.
    ReturnSpec {
        columns: Option<FieldIdx>,
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
        /// Optional declared column list (rename/alias for body result columns).
        columns: Option<FieldIdx>,
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

// ── Function catalog types ────────────────────────────────────────────────────

/// Category of a built-in function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum FunctionCategory {
    Scalar,
    Aggregate,
    Window,
    /// Table-valued function — valid in FROM clauses.
    TableValued,
}

/// Whether a cflag enables or omits the function.
///
/// `#[repr(u8)]` with `Enable=0, Omit=1` matches the C ABI
/// (`0=Enable, 1=Omit` in `SyntaqliteAvailabilityRule.cflag_polarity`),
/// which allows [`AvailabilityRule`] to be cast directly from C data.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CflagPolarity {
    /// Function requires the cflag to be set (`SQLITE_ENABLE`_*).
    Enable = 0,
    /// Function is omitted when the cflag is set (`SQLITE_OMIT`_*).
    Omit = 1,
}

/// Metadata about a built-in function.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FunctionInfo<'a> {
    /// Function name (lowercase).
    pub name: &'a str,
    /// Supported arities. Negative values indicate variadic:
    /// -1 = any number of args, -N = at least N-1 args.
    pub arities: &'a [i16],
    /// Function category.
    pub category: FunctionCategory,
}

/// A version/cflag availability rule for a function.
///
/// `#[repr(C)]` with layout `i32+i32+u32+u8+3pad = 16 bytes` matches
/// `SyntaqliteAvailabilityRule` exactly, so C extension data can be
/// reinterpreted as `&[AvailabilityRule]` without copying.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct AvailabilityRule {
    /// Minimum `SQLite` version (encoded: major*`1_000_000` + minor*`1_000` + patch).
    pub since: i32,
    /// Maximum `SQLite` version (exclusive). 0 means no upper bound.
    pub until: i32,
    /// Cflag bit index, or `u32::MAX` if no cflag required.
    pub cflag_index: u32,
    /// Polarity of the cflag constraint.
    pub cflag_polarity: CflagPolarity,
}

const _: () = {
    assert!(size_of::<AvailabilityRule>() == 16);
};

/// A function entry combining metadata with availability rules.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FunctionEntry<'a> {
    pub info: FunctionInfo<'a>,
    pub availability: &'a [AvailabilityRule],
}

/// Check whether a function entry is available for the given dialect config.
///
/// A function is available if at least one of its availability rules passes.
/// Each rule checks:
/// - version range (`since`/`until`)
/// - cflag constraint: for `Enable` polarity the cflag must be set; for
///   `Omit` polarity the cflag must be clear. Rules with `cflag_index ==
///   u32::MAX` have no cflag constraint.
pub(crate) fn is_function_available(entry: &FunctionEntry<'_>, dialect: &Dialect) -> bool {
    let cflags = dialect.cflags();
    entry.availability.iter().any(|rule| {
        if dialect.version() < SqliteVersion::from_int(rule.since) {
            return false;
        }
        if rule.until != 0 && dialect.version() >= SqliteVersion::from_int(rule.until) {
            return false;
        }
        if rule.cflag_index != u32::MAX {
            let flag_set = cflags.has_index(rule.cflag_index);
            match rule.cflag_polarity {
                CflagPolarity::Enable => {
                    if !flag_set {
                        return false;
                    }
                }
                CflagPolarity::Omit => {
                    if flag_set {
                        return false;
                    }
                }
            }
        }
        true
    })
}

// ── Dialect handle ────────────────────────────────────────────────────────────

/// Type-erased semantic dialect handle: grammar + formatter + semantic data.
///
/// This bundles:
/// - the syntactic [`AnyGrammar`]
/// - formatter bytecode tables
/// - semantic role table (indexed by node tag)
#[derive(Clone, Copy)]
pub(crate) struct AnyDialect {
    grammar: AnyGrammar,

    // Formatter data — Rust-generated statics.
    fmt_strings: &'static [&'static str],
    fmt_enum_display: &'static [u16],
    fmt_ops: &'static [u8],
    fmt_dispatch: &'static [u32],

    // Semantic role table — generated from `semantic { ... }` annotations.
    roles: &'static [SemanticRole],
}

/// Default dialect handle name used throughout the crate.
pub(crate) type Dialect = AnyDialect;

// SAFETY: wraps immutable static data (C grammar + Rust slices).
unsafe impl Send for AnyDialect {}
// SAFETY: wraps immutable static data (C grammar + Rust slices).
unsafe impl Sync for AnyDialect {}

impl AnyDialect {
    /// Construct from grammar + generated static tables.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        grammar: AnyGrammar,
        fmt_strings: &'static [&'static str],
        fmt_enum_display: &'static [u16],
        fmt_ops: &'static [u8],
        fmt_dispatch: &'static [u32],
        roles: &'static [SemanticRole],
    ) -> Self {
        AnyDialect {
            grammar,
            fmt_strings,
            fmt_enum_display,
            fmt_ops,
            fmt_dispatch,
            roles,
        }
    }

    /// The semantic role table for this dialect, indexed by node tag.
    pub(crate) fn roles(&self) -> &'static [SemanticRole] {
        self.roles
    }

    // ── Formatter accessors ──────────────────────────────────────────────

    /// Read the packed `fmt_dispatch` entry for a node tag.
    ///
    /// Returns `Some((ops_slice, op_count))` or `None` if tag is out of range
    /// or has no ops (sentinel `0xFFFF`).
    pub(crate) fn fmt_dispatch(
        &self,
        tag: syntaqlite_syntax::any::AnyNodeTag,
    ) -> Option<(&[u8], usize)> {
        let idx = u32::from(tag) as usize;
        if idx >= self.fmt_dispatch.len() {
            return None;
        }
        let packed = self.fmt_dispatch[idx];
        let offset = (packed >> 16) as u16;
        let length = (packed & 0xFFFF) as u16;
        if offset == 0xFFFF {
            return None;
        }
        let byte_offset = offset as usize * 6;
        let byte_len = length as usize * 6;
        let slice = &self.fmt_ops[byte_offset..byte_offset + byte_len];
        Some((slice, length as usize))
    }

    /// Look up a string from the fmt string table by index.
    #[inline]
    pub(crate) fn fmt_string(&self, idx: u16) -> &'static str {
        let i = idx as usize;
        assert!(
            i < self.fmt_strings.len(),
            "string index {} out of bounds (count={})",
            i,
            self.fmt_strings.len(),
        );
        self.fmt_strings[i]
    }

    /// Look up a value in the enum display table.
    pub(crate) fn fmt_enum_display_val(&self, idx: usize) -> u16 {
        assert!(
            idx < self.fmt_enum_display.len(),
            "enum_display index {idx} out of bounds",
        );
        self.fmt_enum_display[idx]
    }

    /// Whether this dialect has formatter data.
    pub(crate) fn has_fmt_data(&self) -> bool {
        !self.fmt_strings.is_empty()
    }
}

impl std::ops::Deref for AnyDialect {
    type Target = AnyGrammar;
    fn deref(&self) -> &AnyGrammar {
        &self.grammar
    }
}

impl std::ops::DerefMut for AnyDialect {
    fn deref_mut(&mut self) -> &mut AnyGrammar {
        &mut self.grammar
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialect_roles_returns_slice() {
        let d = AnyDialect::new(
            syntaqlite_syntax::typed::grammar().into_raw(),
            &[],
            &[],
            &[],
            &[],
            &[],
        );
        let roles: &[SemanticRole] = d.roles();
        assert!(roles.is_empty());
    }

    #[test]
    fn semantic_role_variants_exist() {
        let _ = SemanticRole::Transparent;
        let _ = SemanticRole::DefineTable {
            name: 0,
            columns: None,
            select: None,
        };
        let _ = SemanticRole::DefineView {
            name: 0,
            columns: None,
            select: 1,
        };
        let _ = SemanticRole::DefineFunction {
            name: 0,
            args: None,
            return_type: None,
        };
        let _ = SemanticRole::ReturnSpec { columns: None };
        let _ = SemanticRole::Import { module: 0 };
    }

    #[test]
    fn field_idx_is_u8() {
        let _: FieldIdx = 42u8;
    }
}

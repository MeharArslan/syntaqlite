// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Dialect handle, semantic role types, and function catalog types.

use std::sync::Arc;

use syntaqlite_syntax::any::AnyGrammar;
pub(crate) use syntaqlite_syntax::util::SqliteVersion;

// ── Semantic role types ───────────────────────────────────────────────────────

/// Index into a node's field array (0-based).
pub type FieldIdx = u8;

/// The kind of relation a `SourceRef` binding introduces into scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    /// Standard SQL table.
    Table,
    /// View — kept separate from `Table` for catalog queries.
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
pub enum SemanticRole {
    // ── Catalog roles ─────────────────────────────────────────────────────
    /// CREATE TABLE statement: registers a table in the catalog.
    DefineTable {
        /// Field index of the table name.
        name: FieldIdx,
        /// Field index of the column-definition list, if present.
        columns: Option<FieldIdx>,
        /// Field index of an AS-SELECT body, if present.
        select: Option<FieldIdx>,
    },
    /// CREATE VIEW statement: registers a view in the catalog.
    DefineView {
        /// Field index of the view name.
        name: FieldIdx,
        /// Field index of the optional declared column list.
        columns: Option<FieldIdx>,
        /// Field index of the SELECT body.
        select: FieldIdx,
    },
    /// CREATE FUNCTION statement: registers a function in the catalog.
    DefineFunction {
        /// Field index of the function name.
        name: FieldIdx,
        /// Field index of the argument list, if present.
        args: Option<FieldIdx>,
        /// Optional field index of a return-type child node.
        /// The accumulator looks up that child's [`SemanticRole`] and dispatches
        /// on [`SemanticRole::ReturnSpec`] to determine if the function is
        /// table-returning.
        return_type: Option<FieldIdx>,
    },
    /// Annotates a return-type descriptor node (e.g. `PerfettoReturnType`).
    ///
    /// `columns` points to a column-list child; a non-null `NodeId` at runtime
    /// means the enclosing function is table-returning. Any dialect with a
    /// dedicated return-type descriptor node can use this role regardless of
    /// how the surrounding syntax is structured.
    ReturnSpec {
        /// Field index of the column list child, or `None` if scalar-returning.
        columns: Option<FieldIdx>,
    },
    /// Module import statement: registers an imported module name.
    Import {
        /// Field index of the module name.
        module: FieldIdx,
    },

    // ── Column-list items — used during define_table column extraction ─────
    /// A single column definition within a CREATE TABLE column list.
    ColumnDef {
        /// Field index of the column name.
        name: FieldIdx,
        /// Field index of the type annotation, if present.
        type_: Option<FieldIdx>,
        /// Field index of the constraint list, if present.
        constraints: Option<FieldIdx>,
    },

    // ── Result columns — used during SELECT column inference ───────────────
    /// A single result column in a SELECT list.
    ResultColumn {
        /// Field index of the flags bitfield (e.g. `STAR = 1`).
        flags: FieldIdx,
        /// Field index of the alias, if present.
        alias: FieldIdx,
        /// Field index of the value expression.
        expr: FieldIdx,
    },

    // ── Expressions ───────────────────────────────────────────────────────
    /// Function/aggregate/window call: validate name and arg count.
    Call {
        /// Field index of the function name.
        name: FieldIdx,
        /// Field index of the argument list.
        args: FieldIdx,
    },
    /// Column reference: validate column and optional table qualifier.
    ColumnRef {
        /// Field index of the column name.
        column: FieldIdx,
        /// Field index of the optional table qualifier.
        table: FieldIdx,
    },

    // ── Sources ───────────────────────────────────────────────────────────
    /// Table/view reference in FROM — adds binding to current scope.
    SourceRef {
        /// The kind of relation being referenced.
        kind: RelationKind,
        /// Field index of the relation name.
        name: FieldIdx,
        /// Field index of the alias, if present.
        alias: FieldIdx,
    },
    /// Subquery in FROM — opens a fresh scope, then binds alias in outer scope.
    ScopedSource {
        /// Field index of the subquery body.
        body: FieldIdx,
        /// Field index of the alias.
        alias: FieldIdx,
    },

    // ── Scope structure ───────────────────────────────────────────────────
    /// SELECT statement: process `from` first, then validate `exprs`.
    Query {
        /// Field index of the FROM clause.
        from: FieldIdx,
        /// Field index of the result-column list.
        columns: FieldIdx,
        /// Field index of the WHERE clause.
        where_clause: FieldIdx,
        /// Field index of the GROUP BY clause.
        groupby: FieldIdx,
        /// Field index of the HAVING clause.
        having: FieldIdx,
        /// Field index of the ORDER BY clause.
        orderby: FieldIdx,
        /// Field index of the LIMIT clause.
        limit_clause: FieldIdx,
    },
    /// CTE definition: binds a name to a subquery body.
    CteBinding {
        /// Field index of the CTE name.
        name: FieldIdx,
        /// Optional declared column list (rename/alias for body result columns).
        columns: Option<FieldIdx>,
        /// Field index of the SELECT body.
        body: FieldIdx,
    },
    /// WITH clause: sequential CTE scope wrapping a main query.
    CteScope {
        /// Field index of the RECURSIVE flag.
        recursive: FieldIdx,
        /// Field index of the CTE binding list.
        bindings: FieldIdx,
        /// Field index of the main query body.
        body: FieldIdx,
    },
    /// CREATE TRIGGER: injects OLD/NEW into the trigger body scope.
    TriggerScope {
        /// Field index of the target table.
        target: FieldIdx,
        /// Field index of the WHEN expression.
        when: FieldIdx,
        /// Field index of the trigger body.
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
#[derive(Debug, Clone, Copy)]
pub(crate) struct AvailabilityRule {
    /// Minimum `SQLite` version (inclusive).
    pub since: SqliteVersion,
    /// Maximum `SQLite` version (exclusive). `None` means no upper bound.
    pub until: Option<SqliteVersion>,
    /// Cflag bit index, or `u32::MAX` if no cflag required.
    pub cflag_index: u32,
    /// Polarity of the cflag constraint.
    pub cflag_polarity: CflagPolarity,
}

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
        if dialect.version() < rule.since {
            return false;
        }
        if let Some(until) = rule.until
            && dialect.version() >= until
        {
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
///
/// Built-in dialects hold `&'static` data directly. Dynamically loaded
/// dialects use `from_raw_parts` which transmutes library-memory pointers to
/// `&'static` and keeps the library alive via an [`Arc`].
#[derive(Clone)]
pub struct AnyDialect {
    grammar: AnyGrammar,

    // Formatter data — Rust-generated statics.
    fmt_strings: &'static [&'static str],
    fmt_enum_display: &'static [u16],
    fmt_ops: &'static [u8],
    fmt_dispatch: &'static [u32],

    // Semantic role table — generated from `semantic { ... }` annotations.
    roles: &'static [SemanticRole],

    /// Rust-side cflag set (all 42 flags, u64 bitset). Distinct from the
    /// C-ABI `SqliteSyntaxFlags` stored in the grammar — this covers
    /// non-parser flags like `SQLITE_ENABLE_MATH_FUNCTIONS`.
    ext_cflags: crate::util::SqliteFlags,

    /// Keeps a dynamically loaded library alive as long as this dialect handle
    /// (or any clone of it) is alive. `None` for built-in dialects.
    _keep_alive: Option<Arc<dyn Send + Sync>>,
}

/// Default dialect handle name used throughout the crate.
pub type Dialect = AnyDialect;

// SAFETY: wraps immutable static data (C grammar + Rust slices).
unsafe impl Send for AnyDialect {}
// SAFETY: wraps immutable static data (C grammar + Rust slices).
unsafe impl Sync for AnyDialect {}

impl AnyDialect {
    /// Construct from grammar + generated static tables (built-in dialects).
    #[expect(clippy::too_many_arguments)]
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
            ext_cflags: crate::util::SqliteFlags::default(),
            _keep_alive: None,
        }
    }

    /// Construct a dialect from dynamically loaded library data.
    ///
    /// Transmutes the provided slice references to `&'static`, which is safe
    /// as long as the underlying data lives at least as long as `keep_alive`.
    /// When the last clone of this `AnyDialect` is dropped, `keep_alive` is
    /// dropped too, which unloads the library.
    ///
    /// # Safety
    /// All data pointers (`fmt_strings`, `fmt_enum_display`, `fmt_ops`,
    /// `fmt_dispatch`, `roles`) must point into memory owned by `keep_alive`
    /// and remain valid for its entire lifetime.
    pub unsafe fn from_raw_parts(
        grammar: AnyGrammar,
        fmt_strings: &[&str],
        fmt_enum_display: &[u16],
        fmt_ops: &[u8],
        fmt_dispatch: &[u32],
        roles: &[SemanticRole],
        keep_alive: Arc<dyn Send + Sync>,
    ) -> Self {
        // SAFETY: caller guarantees the slices live as long as `keep_alive`.
        let (fmt_strings, fmt_enum_display, fmt_ops, fmt_dispatch, roles) = unsafe {
            (
                std::mem::transmute::<&[&str], &'static [&'static str]>(fmt_strings),
                std::mem::transmute::<&[u16], &'static [u16]>(fmt_enum_display),
                std::mem::transmute::<&[u8], &'static [u8]>(fmt_ops),
                std::mem::transmute::<&[u32], &'static [u32]>(fmt_dispatch),
                std::mem::transmute::<&[SemanticRole], &'static [SemanticRole]>(roles),
            )
        };
        AnyDialect {
            grammar,
            fmt_strings,
            fmt_enum_display,
            fmt_ops,
            fmt_dispatch,
            roles,
            ext_cflags: crate::util::SqliteFlags::default(),
            _keep_alive: Some(keep_alive),
        }
    }

    /// Return a copy of this dialect with the given flags replacing the current
    /// compile-time compatibility flags.
    ///
    /// ```rust,ignore
    /// use syntaqlite::util::{SqliteFlag, SqliteFlags};
    /// let dialect = syntaqlite::sqlite_dialect()
    ///     .with_cflags(SqliteFlags::default().with(SqliteFlag::EnableMathFunctions));
    /// ```
    #[must_use]
    pub fn with_cflags(mut self, flags: crate::util::SqliteFlags) -> Self {
        self.ext_cflags = flags;
        self.grammar = self.grammar.with_cflags(flags.into());
        self
    }

    /// Active compile-time compatibility flags set on this dialect.
    pub fn cflags(&self) -> crate::util::SqliteFlags {
        self.ext_cflags
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
            &[], // _keep_alive: None (built-in)
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

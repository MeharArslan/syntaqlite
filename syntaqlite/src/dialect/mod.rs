// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Dialect handle, semantic role types, and function catalog types.

use std::sync::Arc;

use syntaqlite_syntax::any::AnyGrammar;
use syntaqlite_syntax::typed::TypedGrammar;

#[cfg(feature = "dynload")]
use libloading;
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
pub(crate) fn is_function_available(entry: &FunctionEntry<'_>, dialect: &AnyDialect) -> bool {
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

/// Grammar-parameterized semantic dialect handle: grammar + formatter + semantic data.
///
/// This bundles:
/// - a typed grammar handle `G`
/// - formatter bytecode tables
/// - semantic role table (indexed by node tag)
///
/// Use [`AnyDialect`] (= `TypedDialect<AnyGrammar>`) when the grammar is
/// selected at runtime, or [`TypedDialect<G>`] when the grammar type is known
/// statically (e.g. the built-in `SQLite` grammar).
///
/// Convert to [`AnyDialect`] via [`From`] / `.into()`.
#[derive(Clone)]
pub struct TypedDialect<G: TypedGrammar> {
    grammar: G,

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

/// Type-erased dialect handle (grammar erased to [`AnyGrammar`]).
///
/// Analogous to [`syntaqlite_syntax::any::AnyParser`] being
/// `TypedParser<AnyGrammar>`. Most internal infrastructure works with
/// `AnyDialect`; use [`TypedDialect<G>`] at construction time when the grammar
/// type is known statically.
pub type AnyDialect = TypedDialect<AnyGrammar>;

/// SQLite-specific dialect handle — a newtype around [`AnyDialect`].
///
/// Analogous to [`syntaqlite_syntax::Parser`] being a newtype around
/// `TypedParser<Grammar>`. Use [`AnyDialect`] for infrastructure that must
/// work with any dialect; obtain a `Dialect` via [`crate::sqlite_dialect()`]
/// or `Dialect::new()`.
#[cfg(feature = "sqlite")]
#[derive(Clone)]
pub struct Dialect(pub(crate) AnyDialect);

#[cfg(feature = "sqlite")]
impl Dialect {
    /// Returns the default `SQLite` dialect with no extra flags set.
    pub fn new() -> Self {
        Dialect(crate::sqlite::dialect::dialect())
    }

    /// Return a copy of this dialect targeting a specific `SQLite` version.
    #[must_use]
    pub fn with_version(self, version: SqliteVersion) -> Self {
        Dialect(self.0.with_version(version))
    }

    /// Return a copy of this dialect with the given compile-time flags.
    #[must_use]
    pub fn with_cflags(self, flags: crate::util::SqliteFlags) -> Self {
        Dialect(self.0.with_cflags(flags))
    }

    /// Active compile-time compatibility flags set on this dialect.
    pub fn cflags(&self) -> crate::util::SqliteFlags {
        self.0.cflags()
    }

    /// Target `SQLite` version configured on this dialect's grammar handle.
    pub fn version(&self) -> SqliteVersion {
        self.0.version()
    }

    /// C-parser compile-time compatibility flags (parser flags, indices 0–21).
    pub fn syntax_cflags(&self) -> syntaqlite_syntax::util::SqliteSyntaxFlags {
        self.0.syntax_cflags()
    }

    /// Erase into a type-erased [`AnyDialect`].
    pub fn erase(self) -> AnyDialect {
        self.0
    }
}

#[cfg(feature = "sqlite")]
impl Default for Dialect {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "sqlite")]
impl From<Dialect> for AnyDialect {
    fn from(d: Dialect) -> AnyDialect {
        d.0
    }
}

#[cfg(feature = "sqlite")]
impl std::ops::Deref for Dialect {
    type Target = AnyDialect;
    fn deref(&self) -> &AnyDialect {
        &self.0
    }
}

#[cfg(feature = "sqlite")]
impl std::ops::DerefMut for Dialect {
    fn deref_mut(&mut self) -> &mut AnyDialect {
        &mut self.0
    }
}

// SAFETY: TypedDialect wraps immutable static data (C grammar + Rust slices).
unsafe impl<G: TypedGrammar> Send for TypedDialect<G> {}
// SAFETY: TypedDialect wraps immutable static data (C grammar + Rust slices).
unsafe impl<G: TypedGrammar> Sync for TypedDialect<G> {}

impl<G: TypedGrammar> TypedDialect<G> {
    /// Construct from a typed grammar + generated static tables.
    ///
    /// External dialect authors build a `TypedDialect<G>` and convert to
    /// [`AnyDialect`] via `.into()` to pass to `Formatter`, `SemanticAnalyzer`,
    /// or `LspHost`.
    pub fn new(
        grammar: G,
        fmt_strings: &'static [&'static str],
        fmt_enum_display: &'static [u16],
        fmt_ops: &'static [u8],
        fmt_dispatch: &'static [u32],
        roles: &'static [SemanticRole],
    ) -> Self {
        TypedDialect {
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

    /// The typed grammar handle stored in this dialect.
    pub fn grammar(&self) -> &G {
        &self.grammar
    }

    /// The semantic role table for this dialect, indexed by node tag.
    pub(crate) fn roles(&self) -> &'static [SemanticRole] {
        self.roles
    }

    /// Whether this dialect has formatter data.
    pub(crate) fn has_fmt_data(&self) -> bool {
        !self.fmt_strings.is_empty()
    }

    /// Read the packed `fmt_dispatch` entry for a node tag.
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
}

// ── AnyDialect (= TypedDialect<AnyGrammar>) ───────────────────────────────────

impl AnyDialect {
    /// Load a dialect from a shared library (`.so` / `.dylib` / `.dll`).
    ///
    /// Resolves `syntaqlite_<name>_grammar` (or `syntaqlite_grammar` when `name`
    /// is `None`), calls it, and wraps the result in a [`Dialect`] that keeps
    /// the library alive.
    ///
    /// Dropping the last clone of the returned `Dialect` unloads the library.
    ///
    /// Dynamically loaded dialects supply only the parser grammar; no formatter
    /// bytecode or semantic role tables are present. This means `fmt` and
    /// semantic validation are unavailable for dynamic dialects.
    #[cfg(feature = "dynload")]
    pub fn load(path: &str, name: Option<&str>) -> Result<Self, String> {
        // SAFETY: We keep `lib` alive in an `Arc` below so the grammar pointer
        // lives as long as the Dialect.
        let lib = unsafe {
            libloading::Library::new(path).map_err(|e| format!("failed to load {path:?}: {e}"))?
        };

        let symbol = match name {
            Some(n) => format!("syntaqlite_{n}_grammar"),
            None => "syntaqlite_grammar".to_string(),
        };
        // SAFETY: We call the function immediately and drop `func` before `lib`
        // is moved into the Arc, so there is no lifetime overlap issue.
        let raw: syntaqlite_syntax::typed::CGrammar = unsafe {
            let func: libloading::Symbol<
                '_,
                unsafe extern "C" fn() -> syntaqlite_syntax::typed::CGrammar,
            > = lib
                .get(symbol.as_bytes())
                .map_err(|e| format!("symbol {symbol:?} not found in {path:?}: {e}"))?;
            func()
        };

        // SAFETY: `raw.template` points into the shared library kept alive by
        // `keep_alive`. Dropping the last Dialect clone unloads the library.
        let grammar = unsafe { AnyGrammar::new(raw) };
        let keep_alive: Arc<dyn Send + Sync> = Arc::new(lib);

        // TODO(lalitm): dynamic dialects should also load formatter bytecode
        // and semantic role tables from the shared library so that `fmt` and
        // validation work for external dialects.
        Ok(TypedDialect {
            grammar,
            fmt_strings: &[],
            fmt_enum_display: &[],
            fmt_ops: &[],
            fmt_dispatch: &[],
            roles: &[],
            ext_cflags: crate::util::SqliteFlags::default(),
            _keep_alive: Some(keep_alive),
        })
    }

    /// Return a copy of this dialect targeting a specific `SQLite` version.
    #[must_use]
    pub fn with_version(mut self, version: SqliteVersion) -> Self {
        self.grammar = self.grammar.with_version(version);
        self
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

    /// Target `SQLite` version configured on this dialect's grammar handle.
    pub fn version(&self) -> SqliteVersion {
        self.grammar.version()
    }

    /// C-parser compile-time compatibility flags (parser flags, indices 0–21).
    pub fn syntax_cflags(&self) -> syntaqlite_syntax::util::SqliteSyntaxFlags {
        self.grammar.cflags()
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

impl<G: TypedGrammar> TypedDialect<G> {
    /// Erase the grammar type to produce an [`AnyDialect`].
    ///
    /// `From<TypedDialect<G>>` cannot be implemented for [`AnyDialect`] because
    /// it conflicts with the core blanket `impl<T> From<T> for T` when
    /// `G = AnyGrammar`. Use this method instead.
    pub fn erase(self) -> AnyDialect {
        AnyDialect {
            grammar: self.grammar.into(),
            fmt_strings: self.fmt_strings,
            fmt_enum_display: self.fmt_enum_display,
            fmt_ops: self.fmt_ops,
            fmt_dispatch: self.fmt_dispatch,
            roles: self.roles,
            ext_cflags: self.ext_cflags,
            _keep_alive: self._keep_alive,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "sqlite")]
    fn dialect_roles_returns_slice() {
        // Build a typed dialect from the SQLite grammar, then erase to AnyDialect.
        let typed = TypedDialect::new(
            syntaqlite_syntax::typed::grammar(),
            &[],
            &[],
            &[],
            &[],
            &[],
        );
        let d: AnyDialect = typed.erase();
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

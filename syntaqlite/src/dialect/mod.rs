// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Dialect handle, semantic role types, and function catalog types.

use std::sync::Arc;

use syntaqlite_syntax::any::AnyGrammar;
use syntaqlite_syntax::typed::TypedGrammar;

#[cfg(feature = "dynload")]
use libloading;
pub(crate) use syntaqlite_syntax::util::SqliteVersion;

// ── Semantic role types (re-exported from syntaqlite-common) ─────────────────

pub use syntaqlite_common::roles::{FIELD_ABSENT, FieldIdx, RelationKind, SemanticRole};

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
/// - formatter bytecode tables (all C-native raw pointers)
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

    // Formatter data — all C-native raw pointers from dialect.c exports.
    //
    // String table uses CSR encoding: flat `uint8_t` byte buffer + `uint32_t`
    // offsets array with `fmt_str_count + 1` entries (offsets[N] = total byte count).
    // All other arrays are plain flat arrays.
    //
    // Invariant: if non-null, each pointer points into static data (built-in binary)
    // or into a shared library kept alive by `_keep_alive` (dynamic dialects).
    fmt_str_data: *const u8,
    fmt_str_offsets: *const u32, // fmt_str_count + 1 entries
    fmt_str_count: usize,
    fmt_enum_display: *const u16,
    fmt_enum_display_len: usize,
    fmt_ops: *const u8,
    fmt_ops_len: usize,
    fmt_dispatch: *const u32,
    fmt_dispatch_len: usize,

    // Semantic role table — C-generated byte data.
    //
    // Points to an array of `SemanticRole` values generated by the C codegen
    // pipeline (`dialect_roles.h`) and exposed via `syntaqlite_<name>_roles_data()`.
    // Invariant: if non-null, `roles_ptr` points to `roles_len` consecutive valid
    // `SemanticRole` values with the same `#[repr(C, u8)]` layout as this binary.
    roles_ptr: *const SemanticRole,
    roles_len: usize,

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
    /// Construct from a typed grammar with all C-native data pointers.
    ///
    /// All pointer arguments must point to data that lives at least as long as
    /// this `TypedDialect` (use `'static` for built-in dialects).  Pass null
    /// pointers / zero counts to indicate absent data (e.g. in tests or stubs).
    ///
    /// # Safety
    ///
    /// Each non-null pointer must be valid for `count` elements of its type,
    /// properly aligned, and not aliased mutably for the lifetime of this value.
    /// `fmt_str_offsets` must have `fmt_str_count + 1` entries.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn new(
        grammar: G,
        fmt_str_data: *const u8,
        fmt_str_offsets: *const u32,
        fmt_str_count: usize,
        fmt_enum_display: *const u16,
        fmt_enum_display_len: usize,
        fmt_ops: *const u8,
        fmt_ops_len: usize,
        fmt_dispatch: *const u32,
        fmt_dispatch_len: usize,
        roles_ptr: *const SemanticRole,
        roles_len: usize,
    ) -> Self {
        TypedDialect {
            grammar,
            fmt_str_data,
            fmt_str_offsets,
            fmt_str_count,
            fmt_enum_display,
            fmt_enum_display_len,
            fmt_ops,
            fmt_ops_len,
            fmt_dispatch,
            fmt_dispatch_len,
            roles_ptr,
            roles_len,
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
        if self.roles_ptr.is_null() || self.roles_len == 0 {
            return &[];
        }
        // SAFETY: invariant on roles_ptr/roles_len documented on the struct.
        unsafe { std::slice::from_raw_parts(self.roles_ptr, self.roles_len) }
    }

    /// Whether this dialect has formatter data.
    pub(crate) fn has_fmt_data(&self) -> bool {
        !self.fmt_str_data.is_null() && self.fmt_str_count > 0
    }

    /// Read the packed `fmt_dispatch` entry for a node tag.
    pub(crate) fn fmt_dispatch(
        &self,
        tag: syntaqlite_syntax::any::AnyNodeTag,
    ) -> Option<(&[u8], usize)> {
        if self.fmt_dispatch.is_null() {
            return None;
        }
        let idx = u32::from(tag) as usize;
        if idx >= self.fmt_dispatch_len {
            return None;
        }
        // SAFETY: fmt_dispatch is non-null and idx < fmt_dispatch_len.
        let packed = unsafe { *self.fmt_dispatch.add(idx) };
        let offset = (packed >> 16) as u16;
        let length = (packed & 0xFFFF) as u16;
        if offset == 0xFFFF {
            return None;
        }
        let byte_offset = offset as usize * 6;
        let byte_len = length as usize * 6;
        // SAFETY: fmt_ops is valid for fmt_ops_len bytes; dispatch entries are
        // checked at codegen time to stay within bounds.
        let slice = unsafe { std::slice::from_raw_parts(self.fmt_ops.add(byte_offset), byte_len) };
        Some((slice, length as usize))
    }

    /// Look up a string from the fmt string table by index (CSR encoding).
    #[inline]
    pub(crate) fn fmt_string(&self, idx: u16) -> &'static str {
        let i = idx as usize;
        assert!(
            i < self.fmt_str_count,
            "string index {i} out of bounds (count={})",
            self.fmt_str_count
        );
        // SAFETY: fmt_str_offsets has fmt_str_count + 1 entries.
        let (start, end) = unsafe {
            (
                *self.fmt_str_offsets.add(i) as usize,
                *self.fmt_str_offsets.add(i + 1) as usize,
            )
        };
        // SAFETY: fmt_str_data is valid for the full string buffer; bytes are UTF-8 (codegen).
        unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                self.fmt_str_data.add(start),
                end - start,
            ))
        }
    }

    /// Look up a value in the enum display table.
    pub(crate) fn fmt_enum_display_val(&self, idx: usize) -> u16 {
        assert!(
            idx < self.fmt_enum_display_len,
            "enum_display index {idx} out of bounds"
        );
        // SAFETY: fmt_enum_display is valid for fmt_enum_display_len entries.
        unsafe { *self.fmt_enum_display.add(idx) }
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
    /// Also attempts to load `syntaqlite_<name>_roles_data` and
    /// `syntaqlite_<name>_roles_count` for semantic validation; falls back to
    /// empty roles if those symbols are absent.
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
        // Load semantic roles from the dynamic library (optional - falls back to
        // empty if the dialect does not export the roles symbols).
        let roles_data_sym = match name {
            Some(n) => format!("syntaqlite_{n}_roles_data"),
            None => "syntaqlite_roles_data".to_string(),
        };
        let roles_count_sym = match name {
            Some(n) => format!("syntaqlite_{n}_roles_count"),
            None => "syntaqlite_roles_count".to_string(),
        };
        // SAFETY: We call these functions immediately and do not retain the symbol.
        // The returned pointer remains valid as long as the library is loaded,
        // which is ensured by keep_alive below.
        let (roles_ptr, roles_len) = unsafe {
            let data_ptr = lib
                .get::<unsafe extern "C" fn() -> *const u8>(roles_data_sym.as_bytes())
                .ok()
                .map(|f| f());
            let count = lib
                .get::<unsafe extern "C" fn() -> u32>(roles_count_sym.as_bytes())
                .ok()
                .map(|f| f());
            match (data_ptr, count) {
                (Some(ptr), Some(n)) if !ptr.is_null() => (ptr as *const SemanticRole, n as usize),
                _ => (std::ptr::null(), 0),
            }
        };

        // Load fmt tables from the dynamic library (optional).
        let n = name.unwrap_or("");
        macro_rules! sym_ptr {
            ($suffix:expr, $t:ty) => {{
                let sym_name = if n.is_empty() {
                    format!("syntaqlite_{}", $suffix)
                } else {
                    format!("syntaqlite_{n}_{}", $suffix)
                };
                // SAFETY: lib is valid; symbol resolved and called immediately; result is static ptr.
                unsafe {
                    lib.get::<unsafe extern "C" fn() -> *const $t>(sym_name.as_bytes())
                        .ok()
                        .map(|f| f())
                        .unwrap_or(std::ptr::null())
                }
            }};
        }
        macro_rules! sym_u32 {
            ($suffix:expr) => {{
                let sym_name = if n.is_empty() {
                    format!("syntaqlite_{}", $suffix)
                } else {
                    format!("syntaqlite_{n}_{}", $suffix)
                };
                // SAFETY: lib is valid; symbol resolved and called immediately.
                unsafe {
                    lib.get::<unsafe extern "C" fn() -> u32>(sym_name.as_bytes())
                        .ok()
                        .map(|f| f())
                        .unwrap_or(0) as usize
                }
            }};
        }
        let fmt_str_data = sym_ptr!("fmt_string_data", u8);
        let fmt_str_offsets = sym_ptr!("fmt_string_offsets", u32);
        let fmt_str_count = sym_u32!("fmt_string_count");
        let fmt_enum_display = sym_ptr!("fmt_enum_display", u16);
        let fmt_enum_display_len = sym_u32!("fmt_enum_display_count");
        let fmt_ops = sym_ptr!("fmt_ops", u8);
        let fmt_ops_len = sym_u32!("fmt_ops_count");
        let fmt_dispatch = sym_ptr!("fmt_dispatch", u32);
        let fmt_dispatch_len = sym_u32!("fmt_dispatch_count");

        let keep_alive: Arc<dyn Send + Sync> = Arc::new(lib);

        Ok(TypedDialect {
            grammar,
            fmt_str_data,
            fmt_str_offsets,
            fmt_str_count,
            fmt_enum_display,
            fmt_enum_display_len,
            fmt_ops,
            fmt_ops_len,
            fmt_dispatch,
            fmt_dispatch_len,
            roles_ptr,
            roles_len,
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
            fmt_str_data: self.fmt_str_data,
            fmt_str_offsets: self.fmt_str_offsets,
            fmt_str_count: self.fmt_str_count,
            fmt_enum_display: self.fmt_enum_display,
            fmt_enum_display_len: self.fmt_enum_display_len,
            fmt_ops: self.fmt_ops,
            fmt_ops_len: self.fmt_ops_len,
            fmt_dispatch: self.fmt_dispatch,
            fmt_dispatch_len: self.fmt_dispatch_len,
            roles_ptr: self.roles_ptr,
            roles_len: self.roles_len,
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
        // Build a typed dialect from the SQLite grammar with no data, then erase.
        let typed = unsafe {
            TypedDialect::new(
                syntaqlite_syntax::typed::grammar(),
                std::ptr::null(),
                std::ptr::null(),
                0, // fmt strings (CSR)
                std::ptr::null(),
                0, // fmt enum_display
                std::ptr::null(),
                0, // fmt ops
                std::ptr::null(),
                0, // fmt dispatch
                std::ptr::null(),
                0, // roles
            )
        };
        let d: AnyDialect = typed.erase();
        let roles: &[SemanticRole] = d.roles();
        assert!(roles.is_empty());
    }

    #[test]
    fn semantic_role_variants_exist() {
        let _ = SemanticRole::Transparent;
        let _ = SemanticRole::DefineTable {
            name: 0,
            columns: FIELD_ABSENT,
            select: FIELD_ABSENT,
        };
        let _ = SemanticRole::DefineView {
            name: 0,
            columns: FIELD_ABSENT,
            select: 1,
        };
        let _ = SemanticRole::DefineFunction {
            name: 0,
            args: FIELD_ABSENT,
            return_type: FIELD_ABSENT,
        };
        let _ = SemanticRole::ReturnSpec {
            columns: FIELD_ABSENT,
        };
        let _ = SemanticRole::Import { module: 0 };
    }

    #[test]
    fn field_idx_is_u8() {
        let _: FieldIdx = 42u8;
    }
}

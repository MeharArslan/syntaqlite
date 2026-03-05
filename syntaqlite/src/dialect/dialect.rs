// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Core dialect handle types.
//!
//! Mirrors the `syntaqlite-syntax` grammar layout:
//! - [`TypedDialect`]: typed dialect handle contract (`Into<AnyDialect>`)
//! - [`AnyDialect`]: type-erased dialect handle used by infrastructure
//! - [`Dialect`]: convenience alias to [`AnyDialect`]

use syntaqlite_syntax::any::AnyGrammar;
use syntaqlite_syntax::util::SqliteVersion;

use super::catalog::FunctionEntry;
use super::schema::SchemaContribution;

/// Typed dialect contract.
///
/// Typed dialect wrappers should be cheap `Copy` handles and convertible into
/// [`AnyDialect`] for use by grammar-agnostic infrastructure.
pub(crate) trait TypedDialect: Copy + Into<AnyDialect> {}

impl<T> TypedDialect for T where T: Copy + Into<AnyDialect> {}

/// Type-erased semantic dialect handle: grammar + formatter + semantic data.
///
/// This bundles:
/// - the syntactic [`AnyGrammar`]
/// - formatter bytecode tables
/// - semantic catalog/schema contributions
#[derive(Clone, Copy)]
pub(crate) struct AnyDialect {
    grammar: AnyGrammar,

    // Formatter data — Rust-generated statics.
    fmt_strings: &'static [&'static str],
    fmt_enum_display: &'static [u16],
    fmt_ops: &'static [u8],
    fmt_dispatch: &'static [u32],

    // Semantic data.
    function_entries: &'static [FunctionEntry<'static>],
    schema_contributions: &'static [SchemaContribution],
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
        function_entries: &'static [FunctionEntry<'static>],
        schema_contributions: &'static [SchemaContribution],
    ) -> Self {
        AnyDialect {
            grammar,
            fmt_strings,
            fmt_enum_display,
            fmt_ops,
            fmt_dispatch,
            function_entries,
            schema_contributions,
        }
    }

    /// Set the target `SQLite` version.
    #[must_use]
    pub(crate) fn with_version(mut self, version: SqliteVersion) -> Self {
        self.grammar = self.grammar.with_version(version);
        self
    }

    /// The target `SQLite` version.
    pub(crate) fn version(&self) -> SqliteVersion {
        self.grammar.version()
    }

    /// The underlying syntax grammar handle.
    pub(crate) fn grammar(&self) -> AnyGrammar {
        self.grammar
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

    // ── Semantic accessors ───────────────────────────────────────────────

    /// Return dialect-provided function extensions.
    pub(crate) fn function_extensions(&self) -> &'static [FunctionEntry<'static>] {
        self.function_entries
    }

    /// Look up a schema contribution for a given node tag.
    pub(crate) fn schema_contribution_for_tag(
        &self,
        tag: syntaqlite_syntax::any::AnyNodeTag,
    ) -> Option<SchemaContribution> {
        let tag_u32 = u32::from(tag);
        self.schema_contributions
            .iter()
            .find(|sc| sc.node_tag == tag_u32)
            .copied()
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

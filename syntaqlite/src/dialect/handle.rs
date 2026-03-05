// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! [`Dialect<'d>`]: the primary semantic dialect handle.
//!
//! A `Dialect<'d>` bundles a syntactic [`AnyGrammar`](syntaqlite_syntax::any::AnyGrammar)
//! with Rust-generated formatter bytecode and semantic data (function catalog,
//! schema contributions). It is `Copy` and cheap (~3 words).
//!
//! Obtain one from a grammar accessor such as
//! `syntaqlite::sqlite::dialect()`, or construct directly with
//! [`Dialect::new`].

use syntaqlite_syntax::any::AnyGrammar;
use syntaqlite_syntax::util::SqliteVersion;

use super::catalog::FunctionEntry;
use super::schema::SchemaContribution;

// ── Dialect handle ───────────────────────────────────────────────────────────

/// A semantic dialect handle: grammar + formatter data + version config.
///
/// Bundles an [`AnyGrammar`](syntaqlite_syntax::any::AnyGrammar) (the syntactic
/// half) with Rust-generated formatter bytecode and dialect-specific semantic
/// data (function catalog, schema contributions). `Copy` and lightweight.
///
/// Build with [`Dialect::new`] at dialect-construction time (e.g. in a
/// `LazyLock`). Use [`with_version`](Self::with_version) to return a
/// configured copy.
#[derive(Clone, Copy)]
pub struct Dialect<'d> {
    grammar: AnyGrammar,

    // Formatter data — Rust-generated statics (see `syntaqlite/src/sqlite/fmt_statics.rs`).
    fmt_strings: &'d [&'d str],
    fmt_enum_display: &'d [u16],
    fmt_ops: &'d [u8],
    fmt_dispatch: &'d [u32],

    // Semantic data.
    function_entries: &'d [FunctionEntry<'d>],
    schema_contributions: &'d [SchemaContribution],
}

// SAFETY: `Dialect` wraps immutable static data (C grammar + Rust slices).
unsafe impl Send for Dialect<'_> {}
unsafe impl Sync for Dialect<'_> {}

impl<'d> Dialect<'d> {
    /// Construct a `Dialect` from a grammar and pre-built Rust statics.
    ///
    /// This is the low-level constructor; prefer using the generated
    /// `syntaqlite::sqlite::dialect()` accessor.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        grammar: AnyGrammar,
        fmt_strings: &'d [&'d str],
        fmt_enum_display: &'d [u16],
        fmt_ops: &'d [u8],
        fmt_dispatch: &'d [u32],
        function_entries: &'d [FunctionEntry<'d>],
        schema_contributions: &'d [SchemaContribution],
    ) -> Self {
        Dialect {
            grammar,
            fmt_strings,
            fmt_enum_display,
            fmt_ops,
            fmt_dispatch,
            function_entries,
            schema_contributions,
        }
    }

    /// Set the target SQLite version, returning a new `Dialect`.
    pub fn with_version(mut self, version: SqliteVersion) -> Self {
        self.grammar = self.grammar.with_version(version);
        self
    }

    /// The target SQLite version.
    pub fn version(&self) -> SqliteVersion {
        self.grammar.version()
    }

    /// The underlying [`AnyGrammar`](syntaqlite_syntax::any::AnyGrammar) handle.
    pub fn grammar(&self) -> AnyGrammar {
        self.grammar
    }

    // ── Formatter accessors ──────────────────────────────────────────────

    /// Read the packed fmt_dispatch entry for a node tag.
    ///
    /// Returns `Some((ops_slice, op_count))` or `None` if tag is out of
    /// range or has no ops (sentinel 0xFFFF).
    pub fn fmt_dispatch(
        &self,
        tag: syntaqlite_syntax::any::AnyNodeTag,
    ) -> Option<(&'d [u8], usize)> {
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
    pub fn fmt_string(&self, idx: u16) -> &'d str {
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
    pub fn fmt_enum_display_val(&self, idx: usize) -> u16 {
        assert!(
            idx < self.fmt_enum_display.len(),
            "enum_display index {} out of bounds",
            idx,
        );
        self.fmt_enum_display[idx]
    }

    /// Whether this dialect has formatter data.
    pub fn has_fmt_data(&self) -> bool {
        !self.fmt_strings.is_empty()
    }

    // ── Semantic accessors ───────────────────────────────────────────────

    /// Return dialect-provided function extensions.
    pub fn function_extensions(&self) -> &'d [FunctionEntry<'d>] {
        self.function_entries
    }

    /// Look up a schema contribution for a given node tag.
    ///
    /// Linear scan — typically very short (< 10 entries).
    pub fn schema_contribution_for_tag(
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

impl<'d> std::ops::Deref for Dialect<'d> {
    type Target = AnyGrammar;
    fn deref(&self) -> &AnyGrammar {
        &self.grammar
    }
}

impl<'d> std::ops::DerefMut for Dialect<'d> {
    fn deref_mut(&mut self) -> &mut AnyGrammar {
        &mut self.grammar
    }
}

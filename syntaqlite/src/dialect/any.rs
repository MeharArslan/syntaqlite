// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Core dialect handle types.
//!
//! Mirrors the `syntaqlite-syntax` grammar layout:
//! - [`AnyDialect`]: type-erased dialect handle used by infrastructure
//! - [`Dialect`]: convenience alias to [`AnyDialect`]

use syntaqlite_syntax::any::AnyGrammar;

/// Type-erased semantic dialect handle: grammar + formatter + semantic data.
///
/// This bundles:
/// - the syntactic [`AnyGrammar`]
/// - formatter bytecode tables
#[derive(Clone, Copy)]
pub(crate) struct AnyDialect {
    grammar: AnyGrammar,

    // Formatter data — Rust-generated statics.
    fmt_strings: &'static [&'static str],
    fmt_enum_display: &'static [u16],
    fmt_ops: &'static [u8],
    fmt_dispatch: &'static [u32],
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
    ) -> Self {
        AnyDialect {
            grammar,
            fmt_strings,
            fmt_enum_display,
            fmt_ops,
            fmt_dispatch,
        }
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

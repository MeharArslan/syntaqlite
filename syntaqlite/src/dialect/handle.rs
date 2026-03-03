// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! [`Dialect<'d>`]: the primary semantic dialect handle.
//!
//! A `Dialect<'d>` bundles a syntactic [`Grammar<'d>`](syntaqlite_syntax::Grammar)
//! with Rust-generated formatter bytecode and semantic data (function catalog,
//! schema contributions). It is `Copy` and cheap (~3 words).
//!
//! Obtain one from a grammar accessor such as
//! `syntaqlite::sqlite::dialect()`, or construct directly with
//! [`Dialect::new`].

use syntaqlite_syntax::Grammar;
use syntaqlite_syntax::ffi::Cflags;

use super::catalog::FunctionEntry;
use super::schema::SchemaContribution;
use crate::nodes::{FieldVal, Fields, NodeId, SourceSpan};

pub use syntaqlite_syntax::ffi::{
    FIELD_BOOL, FIELD_ENUM, FIELD_NODE_ID, FIELD_SPAN, FieldMeta,
};
pub(crate) use syntaqlite_syntax::ffi::FIELD_FLAGS;

// ── Dialect handle ───────────────────────────────────────────────────────────

/// A semantic dialect handle: grammar + formatter data + version/cflag config.
///
/// Bundles a [`Grammar<'d>`](syntaqlite_syntax::Grammar) (the syntactic
/// half) with Rust-generated formatter bytecode and dialect-specific semantic
/// data (function catalog, schema contributions). `Copy` and lightweight.
///
/// Build with [`Dialect::new`] at dialect-construction time (e.g. in a
/// `LazyLock`). Use [`with_version`](Self::with_version) and
/// [`with_cflag`](Self::with_cflag) to return configured copies.
///
/// For a handle tagged with node/token types, see [`TypedDialect<'d, N>`](super::typed_dialect::TypedDialect).
#[derive(Clone, Copy)]
pub struct Dialect<'d> {
    grammar: Grammar<'d>,

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
        grammar: Grammar<'d>,
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
    pub fn with_version(mut self, version: i32) -> Self {
        self.grammar = self.grammar.with_version(version);
        self
    }

    /// Set a compile-time flag by index, returning a new `Dialect`.
    pub fn with_cflag(mut self, idx: u32) -> Self {
        self.grammar = self.grammar.with_cflag(idx);
        self
    }

    /// Replace the entire cflags bitfield, returning a new `Dialect`.
    pub fn with_cflags(mut self, cflags: Cflags) -> Self {
        self.grammar = self.grammar.with_cflags(cflags);
        self
    }

    /// The target SQLite version.
    pub fn version(&self) -> i32 {
        self.grammar.version()
    }

    /// The active compile-time flags.
    pub fn cflags(&self) -> &Cflags {
        self.grammar.cflags()
    }

    /// The underlying [`Grammar`] handle (syntactic half).
    pub fn grammar(&self) -> Grammar<'d> {
        self.grammar
    }

    /// Build a `ffi::Grammar` FFI struct for passing to C functions.
    pub(crate) fn to_ffi(self) -> syntaqlite_syntax::ffi::Grammar {
        self.grammar.to_ffi()
    }

    // ── Syntactic accessors (delegate to Grammar) ────────────────────────

    /// Return the node name for the given tag.
    pub fn node_name(&self, tag: u32) -> &'d str {
        self.grammar.node_name(tag)
    }

    /// Whether the given node tag represents a list node.
    pub fn is_list(&self, tag: u32) -> bool {
        self.grammar.is_list(tag)
    }

    /// Return the field metadata slice for a node tag.
    pub fn field_meta(&self, tag: u32) -> &'d [FieldMeta] {
        self.grammar.field_meta(tag)
    }

    /// The well-known `TK_SPACE` token type ordinal.
    pub fn tk_space(&self) -> u32 {
        self.grammar.tk_space()
    }

    /// The well-known `TK_SEMI` token type ordinal.
    pub fn tk_semi(&self) -> u32 {
        self.grammar.tk_semi()
    }

    /// The well-known `TK_COMMENT` token type ordinal.
    pub fn tk_comment(&self) -> u32 {
        self.grammar.tk_comment()
    }

    /// Return the raw token category byte for a token type ordinal.
    pub fn token_category_raw(&self, token_type: u32) -> u8 {
        self.grammar.token_category_raw(token_type)
    }

    /// Return number of entries in the grammar's exported keyword table.
    pub fn keyword_count(&self) -> usize {
        self.grammar.keyword_count()
    }

    /// Return the `idx`th keyword entry as `(token_type, keyword_lexeme)`.
    pub fn keyword_entry(&self, idx: usize) -> Option<(u32, &'d str)> {
        self.grammar.keyword_entry(idx)
    }

    /// Return `true` if `name` looks like a completable keyword symbol.
    pub fn is_suggestable_keyword(name: &str) -> bool {
        Grammar::is_suggestable_keyword(name)
    }

    /// Classify a collected token using parser flags, falling back to the
    /// static token-category table.
    pub fn classify_token_raw(&self, token_type: u32, flags: u32) -> u8 {
        use crate::parser::{TOKEN_FLAG_AS_FUNCTION, TOKEN_FLAG_AS_ID, TOKEN_FLAG_AS_TYPE};
        if flags & TOKEN_FLAG_AS_FUNCTION != 0 {
            9 // Function
        } else if flags & TOKEN_FLAG_AS_TYPE != 0 {
            10 // Type
        } else if flags & TOKEN_FLAG_AS_ID != 0 {
            2 // Identifier
        } else {
            self.token_category_raw(token_type)
        }
    }

    // ── Formatter accessors ──────────────────────────────────────────────

    /// Read the packed fmt_dispatch entry for a node tag.
    ///
    /// Returns `Some((ops_slice, op_count))` or `None` if tag is out of
    /// range or has no ops (sentinel 0xFFFF).
    pub fn fmt_dispatch(&self, tag: u32) -> Option<(&'d [u8], usize)> {
        let idx = tag as usize;
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
    pub fn schema_contribution_for_tag(&self, tag: u32) -> Option<SchemaContribution> {
        self.schema_contributions
            .iter()
            .find(|sc| sc.node_tag == tag)
            .copied()
    }
}

// ── Shared field extraction ──────────────────────────────────────────────────

/// Fill a `Fields` buffer by extracting all fields from a raw node pointer.
///
/// # Safety
/// `ptr` must point to a valid node struct matching `tag`'s metadata in `dialect`.
pub(crate) unsafe fn extract_fields<'a>(
    dialect: &Dialect<'_>,
    ptr: *const u8,
    tag: u32,
    source: &'a str,
) -> Fields<'a> {
    let meta = dialect.field_meta(tag);
    let mut fields = Fields::new();
    for m in meta {
        fields.push(unsafe { extract_field_val(ptr, m, source) });
    }
    fields
}

/// Extract a single field value from a raw node pointer using field metadata.
///
/// # Safety
/// `ptr` must point to a valid node struct whose field at `m.offset` has
/// the type indicated by `m.kind`.
pub(crate) unsafe fn extract_field_val<'a>(
    ptr: *const u8,
    m: &FieldMeta,
    source: &'a str,
) -> FieldVal<'a> {
    // SAFETY: All operations below are covered by the function-level safety
    // contract: `ptr` is a valid arena node and `m` describes its field layout.
    unsafe {
        let field_ptr = ptr.add(m.offset as usize);
        match m.kind {
            FIELD_NODE_ID => FieldVal::NodeId(NodeId(*(field_ptr as *const u32))),
            FIELD_SPAN => {
                let span = &*(field_ptr as *const SourceSpan);
                if span.length == 0 {
                    FieldVal::Span("", 0)
                } else {
                    FieldVal::Span(span.as_str(source), span.offset)
                }
            }
            FIELD_BOOL => FieldVal::Bool(*(field_ptr as *const u32) != 0),
            FIELD_FLAGS => FieldVal::Flags(*field_ptr),
            FIELD_ENUM => FieldVal::Enum(*(field_ptr as *const u32)),
            _ => panic!("unknown C field kind: {}", m.kind),
        }
    }
}

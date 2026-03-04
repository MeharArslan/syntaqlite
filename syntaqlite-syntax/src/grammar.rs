// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// TODO(claude): write documentation.

use std::marker::PhantomData;

use crate::ast::NodeFamily;
use crate::cflags::Cflags;

/// A grammar handle tagged with a [`NodeFamily`], carrying both the raw
/// C grammar pointer and the knowledge of which node/token types it produces.
///
/// Use this at construction boundaries (`Parser::new`, etc.) so the
/// node type parameter `N` can be inferred automatically.
///
/// Call [`raw()`](Self::raw) to downgrade to an untyped [`Grammar`] for
/// passing into untyped infrastructure.
#[derive(Clone, Copy)]
pub struct Grammar<N: NodeFamily> {
    inner: RawGrammar,
    _marker: PhantomData<N>,
}

// SAFETY: same reasoning as Grammar — wraps immutable static C data.
unsafe impl<N: NodeFamily> Send for Grammar<N> {}
unsafe impl<N: NodeFamily> Sync for Grammar<N> {}

impl<N: NodeFamily> Grammar<N> {
    /// Build a `Grammar` from a [`RawGrammar`] handle.
    pub fn new(grammar: RawGrammar) -> Self {
        Grammar {
            inner: grammar,
            _marker: PhantomData,
        }
    }

    /// Return the untagged [`RawGrammar`] handle.
    pub fn raw(&self) -> RawGrammar {
        self.inner
    }
}

impl<N: NodeFamily> From<Grammar<N>> for RawGrammar {
    fn from(tg: Grammar<N>) -> Self {
        tg.inner
    }
}

// TODO(claude): add documentation.
#[derive(Clone, Copy)]
pub struct RawGrammar {
    pub(crate) inner: ffi::Grammar,
}

// SAFETY: The grammar wraps an immutable reference to static C data.
unsafe impl Send for RawGrammar {}
unsafe impl Sync for RawGrammar {}

impl RawGrammar {
    // TODO(claude): add a constructor.

    /// Set the target SQLite version.
    pub fn with_version(mut self, version: i32) -> Self {
        self.inner.sqlite_version = version;
        self
    }

    /// Set a compile-time flag by index.
    pub fn with_cflag(mut self, idx: u32) -> Self {
        self.inner.cflags.set(idx);
        self
    }

    /// Replace the entire cflags bitfield.
    pub fn with_cflags(mut self, cflags: Cflags) -> Self {
        self.inner.cflags = cflags;
        self
    }

    /// The target SQLite version.
    pub fn version(&self) -> i32 {
        self.inner.sqlite_version
    }

    /// The active compile-time flags.
    pub fn cflags(&self) -> &Cflags {
        &self.inner.cflags
    }

    // ── Internal helper ──────────────────────────────────────────────────

    /// Return a reference to the abstract grammar template.
    #[inline]
    fn template(&self) -> &'static ffi::GrammarTemplate {
        // SAFETY: `inner.template` points to static C data (generated grammar tables).
        unsafe { &*self.inner.template }
    }

    // ── Metadata accessors ───────────────────────────────────────────────

    /// Return the node name for the given tag.
    pub fn node_name(&self, tag: u32) -> &'static str {
        let raw = self.template();
        let idx = tag as usize;
        assert!(
            idx < raw.node_count as usize,
            "node tag {} out of bounds (count={})",
            idx,
            raw.node_count,
        );
        // SAFETY: idx is bounds-checked above; node_names is a static array of
        // length node_count populated by codegen, with valid NUL-terminated strings.
        unsafe {
            let cstr = std::ffi::CStr::from_ptr(*raw.node_names.add(idx));
            cstr.to_str().expect("invalid UTF-8 in node name")
        }
    }

    /// Whether the given node tag represents a list node.
    pub fn is_list(&self, tag: u32) -> bool {
        let raw = self.template();
        let idx = tag as usize;
        if idx >= raw.node_count as usize {
            return false;
        }
        // SAFETY: idx is bounds-checked above; list_tags is a static array of
        // length node_count populated by codegen.
        unsafe { *raw.list_tags.add(idx) != 0 }
    }

    /// Return the field metadata slice for a node tag.
    pub fn field_meta(&self, tag: u32) -> &'static [ffi::FieldMeta] {
        let raw = self.template();
        let idx = tag as usize;
        if idx >= raw.node_count as usize {
            return &[];
        }
        // SAFETY: idx is bounds-checked above; field_meta_counts and field_meta
        // are parallel arrays of length node_count populated by codegen.
        unsafe {
            let count = *raw.field_meta_counts.add(idx) as usize;
            let ptr = *raw.field_meta.add(idx);
            if count == 0 || ptr.is_null() {
                return &[];
            }
            std::slice::from_raw_parts(ptr, count)
        }
    }

    /// Return the raw token category byte for a token type ordinal.
    pub fn token_category_raw(&self, token_type: u32) -> u8 {
        let raw = self.template();
        if raw.token_categories.is_null() || token_type >= raw.token_type_count {
            return 0;
        }
        // SAFETY: token_type is bounds-checked above; token_categories is a static array.
        unsafe { *raw.token_categories.add(token_type as usize) }
    }

    /// Return number of entries in the grammar's exported mkkeyword table.
    pub fn keyword_count(&self) -> usize {
        let raw = self.template();
        if raw.keyword_count.is_null() {
            return 0;
        }
        // SAFETY: keyword_count is null-checked above; it points to a static u32.
        unsafe { *raw.keyword_count as usize }
    }

    /// Return the `idx`th keyword entry as `(token_type, keyword_lexeme)`.
    pub fn keyword_entry(&self, idx: usize) -> Option<(u32, &'static str)> {
        let raw = self.template();
        if raw.keyword_text.is_null()
            || raw.keyword_offsets.is_null()
            || raw.keyword_lens.is_null()
            || raw.keyword_codes.is_null()
            || raw.keyword_count.is_null()
        {
            return None;
        }
        // SAFETY: all keyword pointers are null-checked above; arrays are static
        // and populated by codegen. idx is bounds-checked against keyword_count.
        unsafe {
            let keyword_count = *raw.keyword_count as usize;
            if idx >= keyword_count {
                return None;
            }
            let code = *raw.keyword_codes.add(idx) as u32;
            let len = *raw.keyword_lens.add(idx) as usize;
            if len == 0 {
                return None;
            }
            let off = *raw.keyword_offsets.add(idx) as usize;
            let text_base = raw.keyword_text as *const u8;
            let bytes = std::slice::from_raw_parts(text_base.add(off), len);
            let value = std::str::from_utf8_unchecked(bytes);
            Some((code, value))
        }
    }

    /// Return `true` if `name` looks like a completable keyword symbol.
    pub fn is_suggestable_keyword(name: &str) -> bool {
        !name.is_empty()
            && name
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
    }
}

pub(crate) mod ffi {
    use crate::cflags::Cflags;

    /// Mirrors C `SyntaqliteGrammarTemplate` struct defined in
    /// `include/syntaqlite/grammar.h`.
    #[repr(C)]
    pub(crate) struct GrammarTemplate {
        pub(crate) name: *const std::ffi::c_char,

        // Range metadata
        pub(crate) range_meta: *const std::ffi::c_void,

        // AST metadata
        pub(crate) node_count: u32,
        pub(crate) node_names: *const *const std::ffi::c_char,
        pub(crate) field_meta: *const *const FieldMeta,
        pub(crate) field_meta_counts: *const u8,
        pub(crate) list_tags: *const u8,

        // Parser lifecycle (function pointers provided by grammar)
        pub(crate) parser_alloc: *const std::ffi::c_void,
        pub(crate) parser_init: *const std::ffi::c_void,
        pub(crate) parser_finalize: *const std::ffi::c_void,
        pub(crate) parser_free: *const std::ffi::c_void,
        pub(crate) parser_feed: *const std::ffi::c_void,
        pub(crate) parser_trace: *const std::ffi::c_void,
        pub(crate) parser_expected_tokens: *const std::ffi::c_void,
        pub(crate) parser_completion_context: *const std::ffi::c_void,

        // Tokenizer (function pointer provided by grammar)
        pub(crate) get_token: *const std::ffi::c_void,

        // Keyword table metadata
        pub(crate) keyword_text: *const std::ffi::c_char,
        pub(crate) keyword_offsets: *const u16,
        pub(crate) keyword_lens: *const u8,
        pub(crate) keyword_codes: *const u8,
        pub(crate) keyword_count: *const u32,

        // Token metadata (indexed by token type ordinal)
        pub(crate) token_categories: *const u8,
        pub(crate) token_type_count: u32,
    }

    /// Mirrors C `SyntaqliteGrammar` from `include/syntaqlite/grammar.h`.
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub(crate) struct Grammar {
        pub(crate) template: *const GrammarTemplate,
        pub(crate) sqlite_version: i32,
        pub(crate) cflags: Cflags,
    }

    /// Mirrors C `SyntaqliteFieldMeta` from `include/syntaqlite_dialect/dialect_types.h`.
    #[repr(C)]
    pub struct FieldMeta {
        pub offset: u16,
        pub kind: u8,
        pub name: *const std::ffi::c_char,
        pub display: *const *const std::ffi::c_char,
        pub display_count: u8,
    }
}

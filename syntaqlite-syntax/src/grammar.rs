// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use crate::cflags::Cflags;

// ── Public API ───────────────────────────────────────────────────────────────

/// A typed grammar handle for a specific dialect.
///
/// Implement this for a dialect's grammar struct (e.g. `SqliteGrammar`).
/// The struct carries a [`AnyGrammar`] internally and exposes it via [`raw`](Self::raw).
pub trait TypedGrammar: Copy {
    /// The top-level typed AST node enum for this dialect.
    type Node<'a>: crate::ast::GrammarNodeType<'a>;
    /// The typed token enum for this dialect.
    type Token: crate::ast::GrammarTokenType;
    /// Access the underlying [`AnyGrammar`] for FFI and configuration.
    fn raw(&mut self) -> &mut AnyGrammar;
}

// TODO(claude): add documentation.
#[derive(Clone, Copy)]
pub struct AnyGrammar {
    pub(crate) inner: ffi::CGrammar,
}

// SAFETY: The grammar wraps an immutable reference to static C data.
unsafe impl Send for AnyGrammar {}
unsafe impl Sync for AnyGrammar {}

impl AnyGrammar {
    /// Construct a `AnyGrammar` from a raw C grammar value.
    ///
    /// # Safety
    /// The `template` pointer inside `inner` must point to valid, `'static`
    /// C grammar tables (e.g. returned by a dialect's `extern "C"` grammar
    /// accessor such as `syntaqlite_sqlite_grammar()`).
    pub unsafe fn new(inner: ffi::CGrammar) -> Self {
        AnyGrammar { inner }
    }

    /// Set the target SQLite version.
    pub fn with_version(mut self, version: crate::util::SqliteVersion) -> Self {
        self.inner.sqlite_version = version.as_int();
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

    /// Return a reference to the abstract grammar template.
    #[inline]
    fn template(&self) -> &'static ffi::CGrammarTemplate {
        // SAFETY: `inner.template` points to static C data (generated grammar tables).
        unsafe { &*self.inner.template }
    }

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
    pub fn field_meta(&self, tag: u32) -> &'static [ffi::CFieldMeta] {
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

// ── Crate-internal ───────────────────────────────────────────────────────────

pub(crate) use ffi::CFieldMeta as FieldMeta;

// ── ffi ───────────────────────────────────────────────────────────────────────

pub(crate) mod ffi {
    use crate::cflags::Cflags;

    /// Mirrors C `SyntaqliteGrammarTemplate` struct defined in
    /// `include/syntaqlite/grammar.h`.
    #[repr(C)]
    pub(crate) struct CGrammarTemplate {
        pub(crate) name: *const std::ffi::c_char,

        // Range metadata
        pub(crate) range_meta: *const std::ffi::c_void,

        // AST metadata
        pub(crate) node_count: u32,
        pub(crate) node_names: *const *const std::ffi::c_char,
        pub(crate) field_meta: *const *const CFieldMeta,
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
    pub(crate) struct CGrammar {
        pub(crate) template: *const CGrammarTemplate,
        pub(crate) sqlite_version: i32,
        pub(crate) cflags: Cflags,
    }

    /// Mirrors C `SyntaqliteFieldMeta` from `include/syntaqlite_dialect/dialect_types.h`.
    #[repr(C)]
    pub struct CFieldMeta {
        pub offset: u16,
        pub kind: u8,
        pub name: *const std::ffi::c_char,
        pub display: *const *const std::ffi::c_char,
        pub display_count: u8,
    }

    impl CFieldMeta {
        /// Return the field name as a `&str`.
        ///
        /// # Safety
        /// The `name` pointer must be valid and NUL-terminated for the lifetime
        /// of the returned `&str`.
        pub unsafe fn name_str(&self) -> &str {
            unsafe {
                let cstr = std::ffi::CStr::from_ptr(self.name);
                cstr.to_str().expect("invalid UTF-8 in field name")
            }
        }

        /// Return the display string for an enum ordinal or flag bit index.
        ///
        /// Returns `None` if `idx` is out of range or the entry is null.
        ///
        /// # Safety
        /// The `display` pointer must be valid for `display_count` entries,
        /// each pointing to a NUL-terminated C string (or null).
        pub unsafe fn display_name(&self, idx: usize) -> Option<&str> {
            if self.display.is_null() || idx >= self.display_count as usize {
                return None;
            }
            unsafe {
                let ptr = *self.display.add(idx);
                if ptr.is_null() {
                    return None;
                }
                let cstr = std::ffi::CStr::from_ptr(ptr);
                Some(cstr.to_str().unwrap_or("?"))
            }
        }
    }
}

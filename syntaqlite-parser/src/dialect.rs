// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Dialect handle and field extraction.
//!
//! A [`Dialect`] is an opaque, `Copy` handle wrapping a pointer to a C
//! dialect descriptor produced by codegen. It provides metadata about
//! node names, field layouts, token categories, keyword tables, and
//! formatter bytecode — everything a parser, formatter, or validator needs
//! to operate on a particular SQL grammar.

pub mod ffi {
    //! C ABI mirror structs for the dialect.

    pub const FIELD_NODE_ID: u8 = 0;
    pub const FIELD_SPAN: u8 = 1;
    pub const FIELD_BOOL: u8 = 2;
    pub const FIELD_FLAGS: u8 = 3;
    pub const FIELD_ENUM: u8 = 4;

    /// Mirrors C `SyntaqliteCflags` from `include/syntaqlite/cflags.h`.
    ///
    /// A packed bitfield struct. On the Rust side we represent it as raw bytes
    /// and provide index-based accessors matching the C `SYNQ_CFLAG_IDX_*` constants.
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct Cflags {
        bytes: [u8; 6],
    }

    impl Cflags {
        /// Create a zero-initialized (all flags off) cflags.
        pub const fn new() -> Self {
            Self { bytes: [0; 6] }
        }

        /// Check if cflag at `idx` is set.
        #[inline]
        pub fn has(&self, idx: u32) -> bool {
            let byte = idx / 8;
            let bit = idx % 8;
            (byte < 6) && (self.bytes[byte as usize] >> bit) & 1 != 0
        }

        /// Set cflag at `idx`.
        #[inline]
        pub fn set(&mut self, idx: u32) {
            let byte = idx / 8;
            let bit = idx % 8;
            if byte < 6 {
                self.bytes[byte as usize] |= 1 << bit;
            }
        }

        /// Clear cflag at `idx`.
        #[inline]
        pub fn clear(&mut self, idx: u32) {
            let byte = idx / 8;
            let bit = idx % 8;
            if byte < 6 {
                self.bytes[byte as usize] &= !(1 << bit);
            }
        }

        /// Reset all cflags to zero.
        #[inline]
        pub fn clear_all(&mut self) {
            self.bytes = [0; 6];
        }
    }

    impl Default for Cflags {
        fn default() -> Self {
            Self::new()
        }
    }

    impl std::fmt::Debug for Cflags {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Cflags({:02x?})", &self.bytes)
        }
    }

    /// Mirrors C `SyntaqliteDialectConfig` from `include/syntaqlite/dialect.h`.
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct DialectConfig {
        pub sqlite_version: i32,
        pub cflags: Cflags,
    }

    impl Default for DialectConfig {
        fn default() -> Self {
            Self {
                sqlite_version: i32::MAX,
                cflags: Cflags::new(),
            }
        }
    }

    // ── Function extension FFI mirrors ─────────────────────────────────────

    /// Mirrors C `SyntaqliteFunctionInfo` from `include/syntaqlite/dialect.h`.
    #[repr(C)]
    pub struct FunctionInfoC {
        pub name: *const std::ffi::c_char,
        pub arities: *const i16,
        pub arity_count: u16,
        pub category: u8,
    }

    /// Mirrors C `SyntaqliteAvailabilityRule` from `include/syntaqlite/dialect.h`.
    #[repr(C)]
    pub struct AvailabilityRuleC {
        pub since: i32,
        pub until: i32,
        pub cflag_index: u32,
        pub cflag_polarity: u8,
    }

    /// Mirrors C `SyntaqliteFunctionEntry` from `include/syntaqlite/dialect.h`.
    #[repr(C)]
    pub struct FunctionEntryC {
        pub info: FunctionInfoC,
        pub availability: *const AvailabilityRuleC,
        pub availability_count: u16,
    }

    /// Mirrors C `SyntaqliteSchemaContribution` from `include/syntaqlite/dialect.h`.
    #[repr(C)]
    pub struct SchemaContributionC {
        pub node_tag: u32,
        pub kind: u8,
        pub name_field: u8,
        pub columns_field: u8,
        pub select_field: u8,
        pub args_field: u8,
        pub _pad: [u8; 3],
    }

    // Layout assertions for FFI mirrors.
    const _: () = {
        assert!(std::mem::size_of::<AvailabilityRuleC>() == 16);
        assert!(std::mem::size_of::<SchemaContributionC>() == 12);
        #[cfg(target_pointer_width = "64")]
        assert!(std::mem::size_of::<Dialect>() == 296);
        #[cfg(target_pointer_width = "32")]
        assert!(std::mem::size_of::<Dialect>() == 156);
    };

    /// Mirrors C `SyntaqliteFieldMeta` from `include/syntaqlite/dialect.h`.
    #[repr(C)]
    pub struct FieldMeta {
        pub offset: u16,
        pub kind: u8,
        pub name: *const std::ffi::c_char,
        pub display: *const *const std::ffi::c_char,
        pub display_count: u8,
    }

    impl FieldMeta {
        /// Return the field name as a `&str`.
        ///
        /// # Safety
        /// The `name` pointer must be valid and NUL-terminated for the lifetime
        /// of the returned `&str`.
        pub unsafe fn name_str(&self) -> &str {
            // SAFETY: caller guarantees that self.name is a valid, NUL-terminated
            // C string pointer for the lifetime of the returned &str.
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
            // SAFETY: caller guarantees display array is valid for display_count entries.
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

    // ── Cflag metadata (pure Rust, no C dependency) ──────────────────────

    /// Metadata for a single compile-time flag.
    #[derive(Debug, Clone)]
    pub struct CflagInfo {
        /// The suffix shared across C and Rust (e.g. `"SQLITE_OMIT_WINDOWFUNC"`).
        ///
        /// - C define: `SYNTAQLITE_CFLAG_SQLITE_OMIT_WINDOWFUNC`
        /// - Rust env var: `SYNTAQLITE_CFLAG_SQLITE_OMIT_WINDOWFUNC=1`
        pub suffix: String,
        /// Bit index in [`Cflags`] (matches `SYNQ_CFLAG_IDX_*` constants).
        pub index: u32,
        /// Minimum SQLite version (encoded integer) at which this cflag has any
        /// observable effect on keyword recognition. Zero means baseline (all versions).
        pub min_version: i32,
        /// Feature-area category for UI grouping:
        /// `"parser"`, `"functions"`, `"vtable"`, or `"extensions"`.
        pub category: String,
    }

    /// Mirrors the C `Dialect` struct defined in `include/syntaqlite/dialect.h`.
    #[repr(C)]
    pub struct Dialect {
        pub name: *const std::ffi::c_char,

        // Range metadata
        pub range_meta: *const std::ffi::c_void,

        // Well-known token IDs (int32_t in C)
        pub tk_space: i32,
        pub tk_semi: i32,
        pub tk_comment: i32,

        // AST metadata
        pub node_count: u32,
        pub node_names: *const *const std::ffi::c_char,
        pub field_meta: *const *const FieldMeta,
        pub field_meta_counts: *const u8,
        pub list_tags: *const u8,

        // Formatter data
        pub fmt_strings: *const *const std::ffi::c_char,
        pub fmt_string_lens: *const u16,
        pub fmt_string_count: u16,
        pub fmt_enum_display: *const u16,
        pub fmt_enum_display_count: u16,
        pub fmt_ops: *const u8,
        pub fmt_op_count: u16,
        pub fmt_dispatch: *const u32,
        pub fmt_dispatch_count: u16,

        // Parser lifecycle (function pointers provided by dialect)
        pub parser_alloc: *const std::ffi::c_void,
        pub parser_init: *const std::ffi::c_void,
        pub parser_finalize: *const std::ffi::c_void,
        pub parser_free: *const std::ffi::c_void,
        pub parser_feed: *const std::ffi::c_void,
        pub parser_trace: *const std::ffi::c_void,
        pub parser_expected_tokens: *const std::ffi::c_void,
        pub parser_completion_context: *const std::ffi::c_void,

        // Tokenizer (function pointer provided by dialect)
        pub get_token: *const std::ffi::c_void,

        // Keyword table metadata
        pub keyword_text: *const std::ffi::c_char,
        pub keyword_offsets: *const u16,
        pub keyword_lens: *const u8,
        pub keyword_codes: *const u8,
        pub keyword_count: *const u32,

        // Token metadata (indexed by token type ordinal)
        pub token_categories: *const u8,
        pub token_type_count: u32,

        // Dialect function extensions
        pub function_extensions: *const FunctionEntryC,
        pub function_extension_count: u32,

        // Schema contributions
        pub schema_contributions: *const SchemaContributionC,
        pub schema_contribution_count: u32,
    }
}

// Re-export the C types (excluding the `Dialect` C struct to avoid
// naming collision with the safe Rust wrapper below).
pub use ffi::{
    DialectConfig, FIELD_BOOL, FIELD_ENUM, FIELD_FLAGS, FIELD_NODE_ID, FIELD_SPAN, FieldMeta,
};
// Re-export the C `Dialect` struct under a distinct name for external callers
// that need to declare FFI functions returning a raw dialect pointer.
pub use ffi::Dialect as FfiDialect;

use crate::catalog::{AvailabilityRule, FunctionCategory, FunctionEntry, FunctionInfo};
use crate::nodes::{FieldVal, Fields, NodeId, SourceSpan};

// ── Safe Dialect handle ──────────────────────────────────────────────────────

/// An opaque dialect handle wrapping a pointer to a C dialect descriptor.
///
/// Provides metadata about node names, field layouts, token categories,
/// keyword tables, and formatter bytecode. `Copy` so it can be threaded
/// freely through parser, formatter, and validator internals.
#[derive(Clone, Copy)]
pub struct Dialect<'d> {
    pub(crate) raw: &'d ffi::Dialect,
}

impl<'d> Dialect<'d> {
    /// Create a `Dialect` from a raw C pointer returned by a dialect's
    /// FFI function (e.g. `syntaqlite_sqlite_dialect`).
    ///
    /// # Safety
    /// The pointer must point to a valid `ffi::Dialect` whose data lives
    /// at least as long as `'d`.
    pub unsafe fn from_raw(raw: *const ffi::Dialect) -> Self {
        unsafe { Dialect { raw: &*raw } }
    }

    /// Return the node name for the given tag.
    pub fn node_name(&self, tag: u32) -> &'d str {
        let idx = tag as usize;
        assert!(
            idx < self.raw.node_count as usize,
            "node tag {} out of bounds (count={})",
            idx,
            self.raw.node_count,
        );
        // SAFETY: idx is bounds-checked above; node_names is a static array of
        // length node_count populated by codegen, with valid NUL-terminated strings.
        unsafe {
            let cstr = std::ffi::CStr::from_ptr(*self.raw.node_names.add(idx));
            cstr.to_str().expect("invalid UTF-8 in node name")
        }
    }

    /// Whether the given node tag represents a list node.
    pub fn is_list(&self, tag: u32) -> bool {
        let idx = tag as usize;
        if idx >= self.raw.node_count as usize {
            return false;
        }
        // SAFETY: idx is bounds-checked above; list_tags is a static array of
        // length node_count populated by codegen.
        unsafe { *self.raw.list_tags.add(idx) != 0 }
    }

    /// Return the field metadata slice for a node tag.
    pub fn field_meta(&self, tag: u32) -> &'d [ffi::FieldMeta] {
        let idx = tag as usize;
        if idx >= self.raw.node_count as usize {
            return &[];
        }
        // SAFETY: idx is bounds-checked above; field_meta_counts and field_meta
        // are parallel arrays of length node_count populated by codegen.
        unsafe {
            let count = *self.raw.field_meta_counts.add(idx) as usize;
            let ptr = *self.raw.field_meta.add(idx);
            if count == 0 || ptr.is_null() {
                return &[];
            }
            std::slice::from_raw_parts(ptr, count)
        }
    }

    /// Read the packed fmt_dispatch entry for a node tag.
    /// Returns `None` if tag is out of range or has no ops (sentinel 0xFFFF).
    pub fn fmt_dispatch(&self, tag: u32) -> Option<(&'d [u8], usize)> {
        let idx = tag as usize;
        if idx >= self.raw.fmt_dispatch_count as usize {
            return None;
        }
        // SAFETY: idx is bounds-checked above; fmt_dispatch and fmt_ops are
        // static arrays populated by codegen.
        unsafe {
            let packed = *self.raw.fmt_dispatch.add(idx);
            let offset = (packed >> 16) as u16;
            let length = (packed & 0xFFFF) as u16;
            if offset == 0xFFFF {
                return None;
            }
            let byte_offset = offset as usize * 6;
            let byte_len = length as usize * 6;
            let slice = std::slice::from_raw_parts(self.raw.fmt_ops.add(byte_offset), byte_len);
            Some((slice, length as usize))
        }
    }

    /// Look up a string from the C fmt string table by index.
    ///
    /// Uses the precomputed `fmt_string_lens` array to skip `strlen`, and
    /// `from_utf8_unchecked` since all fmt strings are ASCII keywords.
    #[inline]
    pub fn fmt_string(&self, idx: u16) -> &'d str {
        let i = idx as usize;
        assert!(
            i < self.raw.fmt_string_count as usize,
            "string index {} out of bounds (count={})",
            i,
            self.raw.fmt_string_count,
        );
        // SAFETY: i is bounds-checked above; fmt_strings and fmt_string_lens are
        // parallel static arrays populated by codegen. from_utf8_unchecked is safe
        // because all fmt strings are ASCII keywords.
        unsafe {
            let ptr = *self.raw.fmt_strings.add(i);
            let len = *self.raw.fmt_string_lens.add(i) as usize;
            let bytes = std::slice::from_raw_parts(ptr as *const u8, len);
            debug_assert!(
                std::str::from_utf8(bytes).is_ok(),
                "non-UTF-8 in fmt string at index {i}",
            );
            std::str::from_utf8_unchecked(bytes)
        }
    }

    /// Look up a value in the enum display table.
    pub fn fmt_enum_display_val(&self, idx: usize) -> u16 {
        assert!(
            idx < self.raw.fmt_enum_display_count as usize,
            "enum_display index {} out of bounds",
            idx,
        );
        // SAFETY: idx is bounds-checked above; fmt_enum_display is a static array
        // of length fmt_enum_display_count populated by codegen.
        unsafe { *self.raw.fmt_enum_display.add(idx) }
    }

    /// Whether this dialect has formatter data.
    pub fn has_fmt_data(&self) -> bool {
        !self.raw.fmt_strings.is_null() && self.raw.fmt_string_count > 0
    }

    /// Classify a collected token using parser flags, falling back to the
    /// static token-category table.
    ///
    /// The parser annotates tokens with flags like `TOKEN_FLAG_AS_FUNCTION`
    /// when grammar actions identify a keyword used as a function name, type
    /// name, or plain identifier. This method checks those flags first, then
    /// falls back to [`Self::token_category_raw`].
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

    /// Return the raw token category byte for a token type ordinal.
    pub fn token_category_raw(&self, token_type: u32) -> u8 {
        if self.raw.token_categories.is_null() || token_type >= self.raw.token_type_count {
            return 0;
        }
        // SAFETY: token_type is bounds-checked above; token_categories is a static array.
        unsafe { *self.raw.token_categories.add(token_type as usize) }
    }

    /// Return number of entries in the dialect's exported mkkeyword table.
    pub fn keyword_count(&self) -> usize {
        if self.raw.keyword_count.is_null() {
            return 0;
        }
        // SAFETY: keyword_count is null-checked above; it points to a static u32.
        unsafe { *self.raw.keyword_count as usize }
    }

    /// Return the `idx`th keyword entry as `(token_type, keyword_lexeme)`.
    pub fn keyword_entry(&self, idx: usize) -> Option<(u32, &'d str)> {
        if self.raw.keyword_text.is_null()
            || self.raw.keyword_offsets.is_null()
            || self.raw.keyword_lens.is_null()
            || self.raw.keyword_codes.is_null()
            || self.raw.keyword_count.is_null()
        {
            return None;
        }

        // SAFETY: all keyword pointers are null-checked above; arrays are static
        // and populated by codegen. idx is bounds-checked against keyword_count.
        unsafe {
            let keyword_count = *self.raw.keyword_count as usize;
            if idx >= keyword_count {
                return None;
            }
            let code = *self.raw.keyword_codes.add(idx) as u32;
            let len = *self.raw.keyword_lens.add(idx) as usize;
            if len == 0 {
                return None;
            }
            let off = *self.raw.keyword_offsets.add(idx) as usize;
            let text_base = self.raw.keyword_text as *const u8;
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

    /// The well-known `TK_SPACE` token type ordinal.
    pub fn tk_space(&self) -> u32 {
        self.raw.tk_space as u32
    }

    /// The well-known `TK_SEMI` token type ordinal.
    pub fn tk_semi(&self) -> u32 {
        self.raw.tk_semi as u32
    }

    /// The well-known `TK_COMMENT` token type ordinal.
    pub fn tk_comment(&self) -> u32 {
        self.raw.tk_comment as u32
    }

    /// Return dialect-provided function extensions.
    ///
    /// Reads the C vtable's `function_extensions` array and returns a vec of
    /// `FunctionEntry<'d>` that borrow directly from the dialect's static C
    /// data — no allocations, no leaks.
    pub fn function_extensions(&self) -> Vec<FunctionEntry<'d>> {
        if self.raw.function_extensions.is_null() || self.raw.function_extension_count == 0 {
            return Vec::new();
        }
        let count = self.raw.function_extension_count as usize;
        let mut result = Vec::with_capacity(count);
        for i in 0..count {
            // SAFETY: function_extensions is a valid C array of length
            // function_extension_count, all pointing to static dialect data
            // that lives for 'd.
            let entry = unsafe {
                let c_entry = &*self.raw.function_extensions.add(i);

                let cstr = std::ffi::CStr::from_ptr(c_entry.info.name);
                let s = cstr
                    .to_str()
                    .expect("invalid UTF-8 in function extension name");
                // Reborrow with 'd lifetime — the C string lives in static dialect data.
                let name: &'d str = &*(s as *const str);

                let arities: &'d [i16] =
                    if c_entry.info.arities.is_null() || c_entry.info.arity_count == 0 {
                        &[]
                    } else {
                        &*std::ptr::slice_from_raw_parts(
                            c_entry.info.arities,
                            c_entry.info.arity_count as usize,
                        )
                    };

                let category = match c_entry.info.category {
                    1 => FunctionCategory::Aggregate,
                    2 => FunctionCategory::Window,
                    _ => FunctionCategory::Scalar,
                };

                // AvailabilityRule is #[repr(C)] with same layout as
                // AvailabilityRuleC, so reinterpret the pointer directly.
                let availability: &'d [AvailabilityRule] =
                    if c_entry.availability.is_null() || c_entry.availability_count == 0 {
                        &[]
                    } else {
                        &*std::ptr::slice_from_raw_parts(
                            c_entry.availability as *const AvailabilityRule,
                            c_entry.availability_count as usize,
                        )
                    };

                FunctionEntry {
                    info: FunctionInfo {
                        name,
                        arities,
                        category,
                    },
                    availability,
                }
            };
            result.push(entry);
        }
        result
    }

    /// Look up a schema contribution for a given node tag.
    ///
    /// Linear scan of the C array — typically very short (< 10 entries).
    pub fn schema_contribution_for_tag(&self, tag: u32) -> Option<SchemaContribution> {
        if self.raw.schema_contributions.is_null() || self.raw.schema_contribution_count == 0 {
            return None;
        }
        let count = self.raw.schema_contribution_count as usize;
        for i in 0..count {
            // SAFETY: i < count (loop bound); schema_contributions is a static
            // array of length schema_contribution_count populated by codegen.
            let entry = unsafe { &*self.raw.schema_contributions.add(i) };
            if entry.node_tag == tag {
                let opt = |v: u8| if v == 0xFF { None } else { Some(v) };
                return Some(SchemaContribution {
                    kind: match entry.kind {
                        1 => SchemaKind::View,
                        2 => SchemaKind::Function,
                        3 => SchemaKind::Import,
                        _ => SchemaKind::Table,
                    },
                    name_field: entry.name_field,
                    columns_field: opt(entry.columns_field),
                    select_field: opt(entry.select_field),
                    args_field: opt(entry.args_field),
                });
            }
        }
        None
    }
}

// SAFETY: The dialect wraps a reference to a C struct with no mutable state.
// The raw pointers inside ffi::Dialect all point to immutable static data.
unsafe impl Send for Dialect<'_> {}
unsafe impl Sync for Dialect<'_> {}

// ── Schema contribution types ────────────────────────────────────────────────

/// What kind of schema object a node contributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaKind {
    Table,
    View,
    Function,
    Import,
}

/// A schema contribution read from the dialect's C vtable.
#[derive(Debug, Clone, Copy)]
pub struct SchemaContribution {
    pub kind: SchemaKind,
    pub name_field: u8,
    pub columns_field: Option<u8>,
    pub select_field: Option<u8>,
    pub args_field: Option<u8>,
}

// ── Shared field extraction ──────────────────────────────────────────────────

/// Fill a `Fields` buffer by extracting all fields from a raw node pointer.
///
/// # Safety
/// `ptr` must point to a valid node struct matching `tag`'s metadata in `dialect`.
pub unsafe fn extract_fields<'a>(
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
pub unsafe fn extract_field_val<'a>(
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

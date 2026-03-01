// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Dialect handle and token classification.
//!
//! A [`Dialect`] is an opaque, `Copy` handle wrapping a pointer to a C
//! dialect descriptor produced by codegen. It provides metadata about
//! node names, field layouts, token categories, keyword tables, and
//! formatter bytecode — everything a parser, formatter, or validator needs
//! to operate on a particular SQL grammar.
//!
//! Most users will never construct a `Dialect` directly; the built-in
//! SQLite dialect is available via [`sqlite()`].
//! External dialect crates obtain their handle through the generated
//! [`crate::raw::DialectDef`] trait.

pub(crate) mod ffi;

pub use ffi::{CflagInfo, Cflags, DialectConfig, FieldMeta};

#[cfg(feature = "sqlite")]
pub use crate::sqlite::{cflag_names, cflag_table, parse_cflag_name, parse_sqlite_version};

// ── Token category ─────────────────────────────────────────────────────

/// Semantic category for a token type, used for syntax highlighting.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCategory {
    Other = 0,
    Keyword = 1,
    Identifier = 2,
    String = 3,
    Number = 4,
    Operator = 5,
    Punctuation = 6,
    Comment = 7,
    Variable = 8,
    Function = 9,
    Type = 10,
}

/// The semantic token legend: LSP/Monaco token type names in legend-index order.
///
/// This is the single source of truth for the legend. Both the LSP server
/// capabilities and the WASM/Monaco provider must use this same ordering.
pub const SEMANTIC_TOKEN_LEGEND: &[&str] = &[
    "keyword",     // 0
    "variable",    // 1
    "string",      // 2
    "number",      // 3
    "operator",    // 4
    "comment",     // 5
    "punctuation", // 6
    "identifier",  // 7
    "function",    // 8
    "type",        // 9
];

impl TokenCategory {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Keyword,
            2 => Self::Identifier,
            3 => Self::String,
            4 => Self::Number,
            5 => Self::Operator,
            6 => Self::Punctuation,
            7 => Self::Comment,
            8 => Self::Variable,
            9 => Self::Function,
            10 => Self::Type,
            _ => Self::Other,
        }
    }

    /// The LSP semantic token type name for this category.
    /// Returns `None` for `Other` (not emitted as a semantic token).
    pub fn legend_name(self) -> Option<&'static str> {
        let idx = self.legend_index()?;
        Some(SEMANTIC_TOKEN_LEGEND[idx as usize])
    }

    /// Index into [`SEMANTIC_TOKEN_LEGEND`] for this category.
    /// Returns `None` for `Other`.
    pub fn legend_index(self) -> Option<u32> {
        match self {
            Self::Keyword => Some(0),
            Self::Variable => Some(1),
            Self::String => Some(2),
            Self::Number => Some(3),
            Self::Operator => Some(4),
            Self::Comment => Some(5),
            Self::Punctuation => Some(6),
            Self::Identifier => Some(7),
            Self::Function => Some(8),
            Self::Type => Some(9),
            Self::Other => None,
        }
    }
}

// ── Opaque dialect handle ──────────────────────────────────────────────

/// An opaque dialect handle. Dialect crates (e.g. `syntaqlite`) provide a
/// function that returns a `&'static Dialect<'static>` for their grammar.
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
    pub(crate) fn fmt_dispatch(&self, tag: u32) -> Option<(&'d [u8], usize)> {
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
    pub(crate) fn fmt_string(&self, idx: u16) -> &'d str {
        let i = idx as usize;
        assert!(
            i < self.raw.fmt_string_count as usize,
            "string index {} out of bounds (count={})",
            i,
            self.raw.fmt_string_count,
        );
        // SAFETY: i is bounds-checked above (debug_assert); fmt_strings and
        // fmt_string_lens are parallel static arrays populated by codegen.
        // from_utf8_unchecked is safe because all fmt strings are ASCII keywords
        // (validated by debug_assert above in debug builds).
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
    pub(crate) fn fmt_enum_display_val(&self, idx: usize) -> u16 {
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
    pub(crate) fn has_fmt_data(&self) -> bool {
        !self.raw.fmt_strings.is_null() && self.raw.fmt_string_count > 0
    }

    /// Classify a collected token using parser flags, falling back to the
    /// static token-category table.
    ///
    /// The parser annotates tokens with flags like `TOKEN_FLAG_AS_FUNCTION`
    /// when grammar actions identify a keyword used as a function name, type
    /// name, or plain identifier. This method checks those flags first, then
    /// falls back to [`Self::token_category`].
    pub fn classify_token(&self, token_type: u32, flags: u32) -> TokenCategory {
        use crate::parser::ffi::{TOKEN_FLAG_AS_FUNCTION, TOKEN_FLAG_AS_ID, TOKEN_FLAG_AS_TYPE};
        if flags & TOKEN_FLAG_AS_FUNCTION != 0 {
            TokenCategory::Function
        } else if flags & TOKEN_FLAG_AS_TYPE != 0 {
            TokenCategory::Type
        } else if flags & TOKEN_FLAG_AS_ID != 0 {
            TokenCategory::Identifier
        } else {
            self.token_category(token_type)
        }
    }

    /// Classify a token type ordinal into a semantic category.
    pub fn token_category(&self, token_type: u32) -> TokenCategory {
        if self.raw.token_categories.is_null() || token_type >= self.raw.token_type_count {
            return TokenCategory::Other;
        }
        // SAFETY: token_type is bounds-checked above (< token_type_count) and
        // null-checked; token_categories is a static array populated by codegen.
        let byte = unsafe { *self.raw.token_categories.add(token_type as usize) };
        TokenCategory::from_u8(byte)
    }

    /// Return number of entries in the dialect's exported mkkeyword table.
    pub fn keyword_count(&self) -> usize {
        if self.raw.keyword_count.is_null() {
            return 0;
        }
        // SAFETY: keyword_count is null-checked above; it points to a static
        // u32 populated by codegen.
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

        // SAFETY: all keyword pointers are null-checked above; keyword_codes,
        // keyword_lens, keyword_offsets, and keyword_text are static arrays populated
        // by codegen. idx is bounds-checked against keyword_count. keyword_text bytes
        // are ASCII identifiers, so from_utf8_unchecked is safe.
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

    /// The well-known `TK_SPACE` token type ordinal.
    #[allow(dead_code)]
    pub(crate) fn tk_space(&self) -> u32 {
        self.raw.tk_space as u32
    }

    /// The well-known `TK_SEMI` token type ordinal.
    pub(crate) fn tk_semi(&self) -> u32 {
        self.raw.tk_semi as u32
    }

    /// The well-known `TK_COMMENT` token type ordinal.
    #[allow(dead_code)]
    pub(crate) fn tk_comment(&self) -> u32 {
        self.raw.tk_comment as u32
    }

    /// Return dialect-provided function extensions.
    ///
    /// Reads the C vtable's `function_extensions` array and returns a vec of
    /// `FunctionEntry<'d>` that borrow directly from the dialect's static C
    /// data — no allocations, no leaks.
    ///
    /// This is safe because:
    /// - `name` / `arities` point into static C string/array data.
    /// - `availability` is reinterpreted as `&[AvailabilityRule]`: both
    ///   `AvailabilityRuleC` and `AvailabilityRule` are `#[repr(C)]` with
    ///   identical layout (`i32+i32+u32+u8+3pad = 16 bytes`), and
    ///   `CflagPolarity` is `#[repr(u8)]` with `Enable=0, Omit=1` matching
    ///   the C convention.
    pub(crate) fn function_extensions(&self) -> Vec<crate::catalog::FunctionEntry<'d>> {
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
                    1 => crate::catalog::FunctionCategory::Aggregate,
                    2 => crate::catalog::FunctionCategory::Window,
                    _ => crate::catalog::FunctionCategory::Scalar,
                };

                // AvailabilityRule is #[repr(C)] with same layout as
                // AvailabilityRuleC, so reinterpret the pointer directly.
                let availability: &'d [crate::catalog::AvailabilityRule] =
                    if c_entry.availability.is_null() || c_entry.availability_count == 0 {
                        &[]
                    } else {
                        &*std::ptr::slice_from_raw_parts(
                            c_entry.availability as *const crate::catalog::AvailabilityRule,
                            c_entry.availability_count as usize,
                        )
                    };

                crate::catalog::FunctionEntry {
                    info: crate::catalog::FunctionInfo {
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
}

// ── Schema contribution types ──────────────────────────────────────────

#[cfg(feature = "validation")]
/// What kind of schema object a node contributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchemaKind {
    Table,
    View,
    Function,
    Import,
}

#[cfg(feature = "validation")]
/// A schema contribution read from the dialect's C vtable.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SchemaContribution {
    pub(crate) kind: SchemaKind,
    pub(crate) name_field: u8,
    pub(crate) columns_field: Option<u8>,
    pub(crate) select_field: Option<u8>,
    pub(crate) args_field: Option<u8>,
}

#[cfg(feature = "validation")]
impl<'d> Dialect<'d> {
    /// Look up a schema contribution for a given node tag.
    ///
    /// Linear scan of the C array — typically very short (< 10 entries).
    pub(crate) fn schema_contribution_for_tag(&self, tag: u32) -> Option<SchemaContribution> {
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

// ── Shared field extraction ────────────────────────────────────────────

use crate::parser::nodes::{FieldVal, NodeId, SourceSpan};
use ffi::{FIELD_BOOL, FIELD_ENUM, FIELD_FLAGS, FIELD_NODE_ID, FIELD_SPAN};

/// Fill a `Fields` buffer by extracting all fields from a raw node pointer.
///
/// # Safety
/// `ptr` must point to a valid node struct matching `tag`'s metadata in `dialect`.
pub(crate) unsafe fn extract_fields<'a>(
    dialect: &Dialect<'_>,
    ptr: *const u8,
    tag: u32,
    source: &'a str,
) -> crate::parser::nodes::Fields<'a> {
    let meta = dialect.field_meta(tag);
    let mut fields = crate::parser::nodes::Fields::new();
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

/// Return the built-in SQLite dialect handle.
#[cfg(feature = "sqlite")]
pub fn sqlite() -> &'static Dialect<'static> {
    &crate::sqlite::DIALECT
}

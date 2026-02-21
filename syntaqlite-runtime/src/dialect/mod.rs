// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Dialect types: the opaque handle and C ABI mirror structs.

#[doc(hidden)]
pub mod ffi;

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
}

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
            _ => Self::Other,
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
    pub(crate) fn fmt_string(&self, idx: u16) -> &'d str {
        let i = idx as usize;
        assert!(
            i < self.raw.fmt_string_count as usize,
            "string index {} out of bounds (count={})",
            i,
            self.raw.fmt_string_count,
        );
        unsafe {
            let cstr = std::ffi::CStr::from_ptr(*self.raw.fmt_strings.add(i));
            cstr.to_str().expect("invalid UTF-8 in fmt string")
        }
    }

    /// Look up a value in the enum display table.
    pub(crate) fn fmt_enum_display_val(&self, idx: usize) -> u16 {
        assert!(
            idx < self.raw.fmt_enum_display_count as usize,
            "enum_display index {} out of bounds",
            idx,
        );
        unsafe { *self.raw.fmt_enum_display.add(idx) }
    }

    /// Whether this dialect has formatter data.
    pub(crate) fn has_fmt_data(&self) -> bool {
        !self.raw.fmt_strings.is_null() && self.raw.fmt_string_count > 0
    }

    /// Classify a token type ordinal into a semantic category.
    pub fn token_category(&self, token_type: u32) -> TokenCategory {
        if self.raw.token_categories.is_null() || token_type >= self.raw.token_type_count {
            return TokenCategory::Other;
        }
        let byte = unsafe { *self.raw.token_categories.add(token_type as usize) };
        TokenCategory::from_u8(byte)
    }

    /// The well-known `TK_SPACE` token type ordinal.
    pub fn tk_space(&self) -> u32 {
        self.raw.tk_space as u32
    }

    /// The well-known `TK_COMMENT` token type ordinal.
    pub fn tk_comment(&self) -> u32 {
        self.raw.tk_comment as u32
    }
}

// SAFETY: The dialect wraps a reference to a C struct with no mutable state.
// The raw pointers inside ffi::Dialect all point to immutable static data.
unsafe impl Send for Dialect<'_> {}
unsafe impl Sync for Dialect<'_> {}

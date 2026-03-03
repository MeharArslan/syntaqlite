// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! C ABI mirror structs for the grammar descriptor.
//!
//! Mirrors `SyntaqliteGrammarTemplate` and `SyntaqliteGrammar` from
//! `include/syntaqlite/abstract_grammar.h`.

pub const FIELD_NODE_ID: u8 = 0;
pub const FIELD_SPAN: u8 = 1;
pub const FIELD_BOOL: u8 = 2;
pub(crate) const FIELD_FLAGS: u8 = 3;
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

impl FieldMeta {
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

/// Metadata for a single compile-time flag.
#[derive(Debug, Clone)]
pub struct CflagInfo {
    /// The suffix shared across C and Rust (e.g. `"SQLITE_OMIT_WINDOWFUNC"`).
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

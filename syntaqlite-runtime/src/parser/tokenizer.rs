// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::ffi::CStr;

use super::ffi;

/// A raw token: (token_type ordinal, text slice).
#[derive(Debug, Clone, Copy)]
pub struct RawToken<'a> {
    /// The token type as a raw `u32` ordinal (dialect-specific).
    pub token_type: u32,
    /// The text of the token (a slice of the source).
    pub text: &'a str,
}

/// Owns a tokenizer instance. Reusable across inputs via `tokenize()`.
pub struct Tokenizer {
    raw: *mut ffi::Tokenizer,
    /// Null-terminated copy of the source text.
    source_buf: Vec<u8>,
}

// SAFETY: The C tokenizer is self-contained (no thread-local or shared mutable
// state). Moving it between threads is safe; concurrent access is prevented
// by &mut borrowing in tokenize().
unsafe impl Send for Tokenizer {}

impl Tokenizer {
    /// Create a new tokenizer bound to the given dialect.
    pub fn new(dialect: crate::dialect::Dialect<'_>) -> Self {
        let raw =
            unsafe { ffi::syntaqlite_tokenizer_create(std::ptr::null(), dialect.raw as *const _) };
        assert!(!raw.is_null(), "tokenizer allocation failed");
        Tokenizer {
            raw,
            source_buf: Vec::new(),
        }
    }

    /// Bind source text and return a `TokenCursor` for iterating tokens.
    ///
    /// Copies the source into an internal buffer to add a null terminator
    /// (required by the C tokenizer). For zero-copy tokenization, use
    /// [`tokenize_cstr`](Self::tokenize_cstr).
    pub fn tokenize<'a>(&'a mut self, source: &'a str) -> TokenCursor<'a> {
        self.source_buf.clear();
        self.source_buf.reserve(source.len() + 1);
        self.source_buf.extend_from_slice(source.as_bytes());
        self.source_buf.push(0);

        let c_source_ptr = self.source_buf.as_ptr();
        unsafe {
            ffi::syntaqlite_tokenizer_reset(
                self.raw,
                c_source_ptr as *const _,
                source.len() as u32,
            );
        }
        TokenCursor {
            raw: self.raw,
            source,
            c_source_base: c_source_ptr,
        }
    }

    /// Zero-copy variant: bind a null-terminated source and return a
    /// `TokenCursor`.
    ///
    /// The `&CStr` already guarantees a trailing `\0`, so no copy is needed.
    /// The source must be valid UTF-8 (panics otherwise).
    pub fn tokenize_cstr<'a>(&'a mut self, source: &'a CStr) -> TokenCursor<'a> {
        let bytes = source.to_bytes();
        let source_str = std::str::from_utf8(bytes).expect("source must be valid UTF-8");

        unsafe {
            ffi::syntaqlite_tokenizer_reset(self.raw, source.as_ptr(), bytes.len() as u32);
        }
        TokenCursor {
            raw: self.raw,
            source: source_str,
            c_source_base: source.as_ptr() as *const u8,
        }
    }
}

impl Drop for Tokenizer {
    fn drop(&mut self) {
        unsafe { ffi::syntaqlite_tokenizer_destroy(self.raw) }
    }
}

/// An active tokenizer cursor. Iterates tokens from the bound source.
pub struct TokenCursor<'a> {
    raw: *mut ffi::Tokenizer,
    source: &'a str,
    /// Base pointer of the C source buffer. Used to compute offsets back
    /// into the Rust `source` slice.
    c_source_base: *const u8,
}

impl<'a> Iterator for TokenCursor<'a> {
    type Item = RawToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut token = ffi::Token {
            text: std::ptr::null(),
            length: 0,
            type_: 0,
        };
        let rc = unsafe { ffi::syntaqlite_tokenizer_next(self.raw, &mut token) };
        if rc == 0 {
            return None;
        }

        // Compute offset into the source string from the C pointer.
        let offset = token.text as usize - self.c_source_base as usize;
        let len = token.length as usize;
        let text = &self.source[offset..offset + len];

        Some(RawToken {
            token_type: token.type_,
            text,
        })
    }
}

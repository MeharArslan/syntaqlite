// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::ffi::CStr;

use crate::dialect::Dialect;
use syntaqlite_parser::parser::{
    Token, Tokenizer, syntaqlite_tokenizer_create, syntaqlite_tokenizer_destroy,
    syntaqlite_tokenizer_next, syntaqlite_tokenizer_reset, syntaqlite_tokenizer_set_dialect_config,
};

/// A raw token: (token_type ordinal, text slice).
#[derive(Debug, Clone, Copy)]
pub struct RawToken<'a> {
    /// The token type as a raw `u32` ordinal (dialect-specific).
    pub token_type: u32,
    /// The text of the token (a slice of the source).
    pub text: &'a str,
}

/// Owns a tokenizer instance. Reusable across inputs via `tokenize()`.
pub struct RawTokenizer<'d> {
    raw: *mut Tokenizer,
    /// Null-terminated copy of the source text.
    source_buf: Vec<u8>,
    /// Owned dialect config, kept alive so the C pointer remains valid.
    dialect_config: syntaqlite_parser::dialect::ffi::DialectConfig,
    /// Keeps the dialect alive for the lifetime of the tokenizer. The C
    /// tokenizer stores the dialect pointer internally and uses it during
    /// tokenization, so the dialect must outlive this struct.
    _dialect: Dialect<'d>,
}

// SAFETY: The C tokenizer is self-contained (no thread-local or shared mutable
// state). Moving it between threads is safe; concurrent access is prevented
// by &mut borrowing in tokenize().
unsafe impl<'d> Send for RawTokenizer<'d> {}

impl<'d> RawTokenizer<'d> {
    /// Create a new tokenizer for the built-in SQLite dialect.
    #[cfg(feature = "sqlite")]
    pub fn new() -> RawTokenizer<'static> {
        RawTokenizer::builder(*crate::sqlite::DIALECT).build()
    }

    /// Create a builder for a tokenizer bound to the given dialect.
    pub fn builder(dialect: Dialect<'d>) -> RawTokenizerBuilder<'d> {
        RawTokenizerBuilder {
            dialect,
            dialect_config: None,
        }
    }

    /// Bind source text and return a `RawTokenCursor` for iterating tokens.
    ///
    /// Copies the source into an internal buffer to add a null terminator
    /// (required by the C tokenizer). For zero-copy tokenization, use
    /// [`tokenize_cstr`](Self::tokenize_cstr).
    pub fn tokenize<'a>(&'a mut self, source: &'a str) -> RawTokenCursor<'a> {
        self.source_buf.clear();
        self.source_buf.reserve(source.len() + 1);
        self.source_buf.extend_from_slice(source.as_bytes());
        self.source_buf.push(0);

        let c_source_ptr = self.source_buf.as_ptr();
        // SAFETY: self.raw is valid; c_source_ptr points to source_buf which is
        // null-terminated and lives for 'a (borrowed via &'a mut self).
        unsafe {
            syntaqlite_tokenizer_reset(self.raw, c_source_ptr as *const _, source.len() as u32);
        }
        RawTokenCursor {
            raw: self.raw,
            source,
            c_source_base: c_source_ptr,
        }
    }

    /// Zero-copy variant: bind a null-terminated source and return a
    /// `RawTokenCursor`.
    ///
    /// The `&CStr` already guarantees a trailing `\0`, so no copy is needed.
    /// The source must be valid UTF-8 (panics otherwise).
    pub fn tokenize_cstr<'a>(&'a mut self, source: &'a CStr) -> RawTokenCursor<'a> {
        let bytes = source.to_bytes();
        let source_str = std::str::from_utf8(bytes).expect("source must be valid UTF-8");

        // SAFETY: self.raw is valid; source is a CStr (null-terminated, valid for 'a).
        unsafe {
            syntaqlite_tokenizer_reset(self.raw, source.as_ptr(), bytes.len() as u32);
        }
        RawTokenCursor {
            raw: self.raw,
            source: source_str,
            c_source_base: source.as_ptr() as *const u8,
        }
    }
}

#[cfg(feature = "sqlite")]
impl Default for RawTokenizer<'static> {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RawTokenizer<'_> {
    fn drop(&mut self) {
        // SAFETY: self.raw was allocated by syntaqlite_tokenizer_create and has
        // not been freed (Drop runs exactly once).
        unsafe { syntaqlite_tokenizer_destroy(self.raw) }
    }
}

// ── RawTokenizerBuilder ────────────────────────────────────────────────

/// Builder for configuring a [`RawTokenizer`] before construction.
pub struct RawTokenizerBuilder<'d> {
    dialect: Dialect<'d>,
    dialect_config: Option<syntaqlite_parser::dialect::ffi::DialectConfig>,
}

impl<'d> RawTokenizerBuilder<'d> {
    /// Set dialect config for version/cflag-gated tokenization.
    pub fn dialect_config(
        mut self,
        config: syntaqlite_parser::dialect::ffi::DialectConfig,
    ) -> Self {
        self.dialect_config = Some(config);
        self
    }

    /// Build the tokenizer.
    pub fn build(self) -> RawTokenizer<'d> {
        // SAFETY: syntaqlite_tokenizer_create(NULL, dialect) allocates a new
        // tokenizer with default malloc/free. dialect.raw is valid for the call.
        let raw =
            unsafe { syntaqlite_tokenizer_create(std::ptr::null(), self.dialect.raw as *const _) };
        assert!(!raw.is_null(), "tokenizer allocation failed");

        let mut tok = RawTokenizer {
            raw,
            source_buf: Vec::new(),
            dialect_config: syntaqlite_parser::dialect::ffi::DialectConfig::default(),
            _dialect: self.dialect,
        };

        if let Some(config) = self.dialect_config {
            tok.dialect_config = config;
            // SAFETY: tok.raw is valid; we pass a pointer to tok.dialect_config.
            // The C side copies the config value.
            unsafe {
                syntaqlite_tokenizer_set_dialect_config(
                    tok.raw,
                    &tok.dialect_config as *const syntaqlite_parser::dialect::ffi::DialectConfig,
                );
            }
        }

        tok
    }
}

/// An active tokenizer cursor. Iterates raw tokens from the bound source.
pub struct RawTokenCursor<'a> {
    raw: *mut Tokenizer,
    source: &'a str,
    /// Base pointer of the C source buffer. Used to compute offsets back
    /// into the Rust `source` slice.
    c_source_base: *const u8,
}

impl<'a> Iterator for RawTokenCursor<'a> {
    type Item = RawToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut token = Token {
            text: std::ptr::null(),
            length: 0,
            type_: 0,
        };
        // SAFETY: self.raw is valid (owned by RawTokenizer which outlives this
        // RawTokenCursor via the 'a borrow); &mut token is a valid output parameter.
        let rc = unsafe { syntaqlite_tokenizer_next(self.raw, &mut token) };
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

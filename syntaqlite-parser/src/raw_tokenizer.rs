// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::RefCell;
use std::ffi::CStr;
use std::ptr::NonNull;
use std::rc::Rc;

use crate::RawDialect;
use crate::parser::{
    syntaqlite_tokenizer_create, syntaqlite_tokenizer_destroy, syntaqlite_tokenizer_next,
    syntaqlite_tokenizer_reset, syntaqlite_tokenizer_set_dialect_config,
};
use crate::{Token, Tokenizer};

/// A raw token: (token_type ordinal, text slice).
#[derive(Debug, Clone, Copy)]
pub struct RawToken<'a> {
    /// The token type as a raw `u32` ordinal (dialect-specific).
    pub token_type: u32,
    /// The text of the token (a slice of the source).
    pub text: &'a str,
}

/// Holds the C tokenizer handle and mutable state. Checked out by cursors
/// at runtime and returned on [`Drop`].
pub(crate) struct TokenizerInner {
    raw: NonNull<Tokenizer>,
    source_buf: Vec<u8>,
    dialect_config: crate::DialectConfig,
}

impl Drop for TokenizerInner {
    fn drop(&mut self) {
        // SAFETY: self.raw was allocated by syntaqlite_tokenizer_create and has
        // not been freed (Drop runs exactly once).
        unsafe { syntaqlite_tokenizer_destroy(self.raw.as_ptr()) }
    }
}

/// Owns a tokenizer instance. Reusable across inputs via `tokenize()`.
///
/// Uses the same interior-mutability checkout pattern as [`super::RawParser`].
pub struct RawTokenizer<'d> {
    inner: Rc<RefCell<Option<TokenizerInner>>>,
    /// Keeps the dialect alive for the lifetime of the tokenizer. The C
    /// tokenizer stores the dialect pointer internally and uses it during
    /// tokenization, so the dialect must outlive this struct.
    _dialect: RawDialect<'d>,
}

impl<'d> RawTokenizer<'d> {
    /// Create a tokenizer bound to the given dialect with default configuration.
    pub fn new(dialect: impl Into<RawDialect<'d>>) -> Self {
        let dialect = dialect.into();
        // SAFETY: syntaqlite_tokenizer_create(NULL, dialect) allocates a new
        // tokenizer with default malloc/free. dialect.raw is valid for the call.
        let raw = NonNull::new(unsafe {
            syntaqlite_tokenizer_create(std::ptr::null(), dialect.raw as *const _)
        })
        .expect("tokenizer allocation failed");

        let inner = TokenizerInner {
            raw,
            source_buf: Vec::new(),
            dialect_config: crate::DialectConfig::default(),
        };

        RawTokenizer {
            inner: Rc::new(RefCell::new(Some(inner))),
            _dialect: dialect,
        }
    }

    /// Create a tokenizer with a specific dialect config for version/cflag-gated
    /// tokenization.
    pub fn with_dialect_config(
        dialect: impl Into<RawDialect<'d>>,
        config: crate::DialectConfig,
    ) -> Self {
        let tok = Self::new(dialect);
        // Update the config inside the inner.
        if let Some(inner) = tok.inner.borrow_mut().as_mut() {
            inner.dialect_config = config;
            // SAFETY: inner.raw is valid; we pass a pointer to inner.dialect_config.
            // The C side copies the config value.
            unsafe {
                syntaqlite_tokenizer_set_dialect_config(
                    inner.raw.as_ptr(),
                    &inner.dialect_config as *const crate::DialectConfig,
                );
            }
        }
        tok
    }

    /// Bind source text and return a `RawTokenCursor` for iterating tokens.
    ///
    /// Copies the source into an internal buffer to add a null terminator
    /// (required by the C tokenizer). For zero-copy tokenization, use
    /// [`tokenize_cstr`](Self::tokenize_cstr).
    ///
    /// # Panics
    ///
    /// Panics if a cursor from a previous `tokenize()` call is still alive.
    pub fn tokenize<'a>(&self, source: &'a str) -> RawTokenCursor<'a>
    where
        'd: 'a,
    {
        let mut inner = self
            .inner
            .borrow_mut()
            .take()
            .expect("RawTokenizer::tokenize called while a cursor is still active");

        inner.source_buf.clear();
        inner.source_buf.reserve(source.len() + 1);
        inner.source_buf.extend_from_slice(source.as_bytes());
        inner.source_buf.push(0);

        // source_buf has at least one byte (the null terminator just pushed).
        let c_source_ptr =
            NonNull::new(inner.source_buf.as_mut_ptr()).expect("source_buf is non-empty");
        // SAFETY: inner.raw is valid; c_source_ptr points to source_buf which is
        // null-terminated. source_buf lives inside inner which will be owned by
        // the cursor.
        unsafe {
            syntaqlite_tokenizer_reset(
                inner.raw.as_ptr(),
                c_source_ptr.as_ptr() as *const _,
                source.len() as u32,
            );
        }
        RawTokenCursor {
            raw: inner.raw,
            source,
            c_source_base: c_source_ptr,
            inner: Some(inner),
            slot: Rc::clone(&self.inner),
        }
    }

    /// Zero-copy variant: bind a null-terminated source and return a
    /// `RawTokenCursor`.
    ///
    /// The `&CStr` already guarantees a trailing `\0`, so no copy is needed.
    /// The source must be valid UTF-8 (panics otherwise).
    ///
    /// # Panics
    ///
    /// Panics if a cursor from a previous `tokenize()` call is still alive.
    pub fn tokenize_cstr<'a>(&self, source: &'a CStr) -> RawTokenCursor<'a>
    where
        'd: 'a,
    {
        let inner = self
            .inner
            .borrow_mut()
            .take()
            .expect("RawTokenizer::tokenize_cstr called while a cursor is still active");

        let bytes = source.to_bytes();
        let source_str = std::str::from_utf8(bytes).expect("source must be valid UTF-8");

        // SAFETY: inner.raw is valid; source is a CStr (null-terminated, valid for 'a).
        unsafe {
            syntaqlite_tokenizer_reset(inner.raw.as_ptr(), source.as_ptr(), bytes.len() as u32);
        }
        RawTokenCursor {
            raw: inner.raw,
            source: source_str,
            c_source_base: NonNull::new(source.as_ptr() as *mut u8).expect("CStr is non-null"),
            inner: Some(inner),
            slot: Rc::clone(&self.inner),
        }
    }
}

/// An active tokenizer cursor. Iterates raw tokens from the bound source.
///
/// On drop, the checked-out tokenizer state is returned to the parent
/// [`RawTokenizer`].
pub struct RawTokenCursor<'a> {
    raw: NonNull<Tokenizer>,
    source: &'a str,
    /// Base pointer of the C source buffer. Used to compute offsets back
    /// into the Rust `source` slice.
    c_source_base: NonNull<u8>,
    /// Checked-out tokenizer state. Returned to `slot` on drop.
    inner: Option<TokenizerInner>,
    /// Slot to return `inner` to when this cursor is dropped.
    slot: Rc<RefCell<Option<TokenizerInner>>>,
}

impl Drop for RawTokenCursor<'_> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            *self.slot.borrow_mut() = Some(inner);
        }
    }
}

impl<'a> Iterator for RawTokenCursor<'a> {
    type Item = RawToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut token = Token {
            text: std::ptr::null(),
            length: 0,
            type_: 0,
        };
        // SAFETY: self.raw is valid (owned by TokenizerInner in self.inner);
        // &mut token is a valid output parameter.
        let rc = unsafe { syntaqlite_tokenizer_next(self.raw.as_ptr(), &mut token) };
        if rc == 0 {
            return None;
        }

        // Compute offset into the source string from the C pointer.
        let offset = token.text as usize - self.c_source_base.as_ptr() as usize;
        let len = token.length as usize;
        let text = &self.source[offset..offset + len];

        Some(RawToken {
            token_type: token.type_,
            text,
        })
    }
}

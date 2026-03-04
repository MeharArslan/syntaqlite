// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::RefCell;
use std::ffi::CStr;
use std::ptr::NonNull;
use std::rc::Rc;

use crate::grammar::RawGrammar;

/// A raw token produced by the tokenizer: type ordinal + source text slice.
#[derive(Debug, Clone, Copy)]
pub struct Token<'a> {
    /// Raw token type ordinal (dialect-specific).
    pub token_type: u32,
    /// Slice of the source text covered by this token.
    pub text: &'a str,
}

/// Holds the C tokenizer handle and mutable state. Checked out by cursors
/// at runtime and returned on [`Drop`].
pub(crate) struct TokenizerInner {
    raw: NonNull<ffi::CTokenizer>,
    source_buf: Vec<u8>,
}

impl Drop for TokenizerInner {
    fn drop(&mut self) {
        // SAFETY: self.raw was allocated by syntaqlite_tokenizer_create and has
        // not been freed (Drop runs exactly once).
        unsafe { ffi::syntaqlite_tokenizer_destroy(self.raw.as_ptr()) }
    }
}

/// Owns a tokenizer instance. Reusable across inputs via `tokenize()`.
///
/// Uses an interior-mutability checkout pattern: `tokenize()` checks out the
/// C tokenizer state at runtime, and the returned [`TokenCursor`] returns it on
/// drop. This allows `tokenize()` to take `&self` rather than `&mut self`.
pub struct Tokenizer {
    inner: Rc<RefCell<Option<TokenizerInner>>>,
}

impl Tokenizer {
    /// Create a tokenizer bound to the given grammar.
    pub fn new(grammar: RawGrammar) -> Self {
        // SAFETY: syntaqlite_tokenizer_create(NULL, grammar.inner) allocates a new
        // tokenizer with default malloc/free. The C side copies the grammar.
        let raw = NonNull::new(unsafe {
            ffi::syntaqlite_tokenizer_create(std::ptr::null(), grammar.inner)
        })
        .expect("tokenizer allocation failed");

        let inner = TokenizerInner {
            raw,
            source_buf: Vec::new(),
        };
        Tokenizer {
            inner: Rc::new(RefCell::new(Some(inner))),
        }
    }

    /// Bind source text and return a [`TokenCursor`] for iterating tokens.
    ///
    /// Copies the source into an internal buffer to add a null terminator
    /// (required by the C tokenizer). The cursor owns the copy, so the
    /// original `source` does not need to outlive the cursor. For zero-copy
    /// tokenization, use [`tokenize_cstr`](Self::tokenize_cstr).
    ///
    /// # Panics
    ///
    /// Panics if a cursor from a previous `tokenize()` call is still alive.
    pub fn tokenize<'a>(&self, source: &'a str) -> TokenCursor<'a> {
        let mut inner = self
            .inner
            .borrow_mut()
            .take()
            .expect("Tokenizer::tokenize called while a cursor is still active");

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
            ffi::syntaqlite_tokenizer_reset(
                inner.raw.as_ptr(),
                c_source_ptr.as_ptr() as *const _,
                source.len() as u32,
            );
        }
        TokenCursor {
            raw: inner.raw,
            source,
            c_source_base: c_source_ptr,
            inner: Some(inner),
            slot: Rc::clone(&self.inner),
        }
    }

    /// Zero-copy variant: bind a null-terminated source and return a
    /// [`TokenCursor`].
    ///
    /// The `&CStr` already guarantees a trailing `\0`, so no copy is needed.
    /// The source must be valid UTF-8 (panics otherwise).
    ///
    /// # Panics
    ///
    /// Panics if a cursor from a previous `tokenize()` call is still alive,
    /// or if `source` is not valid UTF-8.
    pub fn tokenize_cstr<'a>(&self, source: &'a CStr) -> TokenCursor<'a> {
        let inner = self
            .inner
            .borrow_mut()
            .take()
            .expect("Tokenizer::tokenize_cstr called while a cursor is still active");

        let bytes = source.to_bytes();
        let source_str = std::str::from_utf8(bytes).expect("source must be valid UTF-8");

        // SAFETY: inner.raw is valid; source is a CStr (null-terminated, valid for 'a).
        unsafe {
            ffi::syntaqlite_tokenizer_reset(
                inner.raw.as_ptr(),
                source.as_ptr(),
                bytes.len() as u32,
            );
        }
        TokenCursor {
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
/// [`Tokenizer`].
pub struct TokenCursor<'a> {
    raw: NonNull<ffi::CTokenizer>,
    source: &'a str,
    /// Base pointer of the C source buffer. Used to compute offsets back
    /// into the Rust `source` slice.
    c_source_base: NonNull<u8>,
    /// Checked-out tokenizer state. Returned to `slot` on drop.
    inner: Option<TokenizerInner>,
    /// Slot to return `inner` to when this cursor is dropped.
    slot: Rc<RefCell<Option<TokenizerInner>>>,
}

impl Drop for TokenCursor<'_> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            *self.slot.borrow_mut() = Some(inner);
        }
    }
}

impl<'a> Iterator for TokenCursor<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut token = ffi::CToken {
            text: std::ptr::null(),
            length: 0,
            type_: 0,
        };
        // SAFETY: self.raw is valid (owned by TokenizerInner in self.inner);
        // &mut token is a valid output parameter.
        let rc = unsafe { ffi::syntaqlite_tokenizer_next(self.raw.as_ptr(), &mut token) };
        if rc == 0 {
            return None;
        }

        // Compute offset into the source string from the C pointer.
        let offset = token.text as usize - self.c_source_base.as_ptr() as usize;
        let len = token.length as usize;
        let text = &self.source[offset..offset + len];

        Some(Token {
            token_type: token.type_,
            text,
        })
    }
}

mod ffi {
    use std::ffi::{c_char, c_int};

    /// Opaque C tokenizer type.
    pub(crate) enum CTokenizer {}

    /// A single token produced by the C tokenizer.
    ///
    /// Mirrors C `SyntaqliteToken` from `include/syntaqlite/tokenizer.h`.
    #[repr(C)]
    pub(crate) struct CToken {
        pub(crate) text: *const c_char,
        pub(crate) length: u32,
        pub(crate) type_: u32,
    }

    unsafe extern "C" {
        pub(crate) fn syntaqlite_tokenizer_create(
            mem: *const std::ffi::c_void,
            grammar: crate::grammar::ffi::Grammar,
        ) -> *mut CTokenizer;
        pub(crate) fn syntaqlite_tokenizer_reset(
            tok: *mut CTokenizer,
            source: *const c_char,
            len: u32,
        );
        pub(crate) fn syntaqlite_tokenizer_next(tok: *mut CTokenizer, out: *mut CToken) -> c_int;
        pub(crate) fn syntaqlite_tokenizer_destroy(tok: *mut CTokenizer);
    }
}

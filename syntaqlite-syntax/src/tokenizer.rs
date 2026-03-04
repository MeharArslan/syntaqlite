// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::RefCell;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::rc::Rc;

use crate::ast::{AnyDialect, GrammarTokenType};
use crate::grammar::{AnyGrammar, TypedGrammar};

#[cfg(feature = "sqlite")]
use crate::sqlite::grammar::SqliteGrammar;
#[cfg(feature = "sqlite")]
use crate::sqlite::tokens::TokenType;

// ── Public API ───────────────────────────────────────────────────────────────

/// A tokenizer for the `SQLite` dialect.
///
/// Yields [`TypedToken`]s with SQLite-specific token types. For other dialects
/// use [`TypedTokenizer`] directly; for dialect-agnostic use with raw `u32`
/// ordinals use [`AnyTokenizer`].
#[cfg(feature = "sqlite")]
pub struct Tokenizer(TypedTokenizer<SqliteGrammar>);

#[cfg(feature = "sqlite")]
impl Tokenizer {
    /// Create a tokenizer for the `SQLite` dialect.
    pub fn new() -> Self {
        Tokenizer(TypedTokenizer::new(crate::sqlite::grammar::grammar()))
    }

    /// Bind source text and return an iterator over `SQLite` tokens.
    ///
    /// # Panics
    ///
    /// Panics if a cursor from a previous `tokenize()` call is still alive.
    pub fn tokenize<'a>(&self, source: &'a str) -> impl Iterator<Item = Token<'a>> {
        self.0.tokenize(source).map(Token)
    }

    /// Zero-copy variant: bind a null-terminated source.
    ///
    /// # Panics
    ///
    /// Panics if a cursor from a previous `tokenize()` call is still alive,
    /// or if `source` is not valid UTF-8.
    pub fn tokenize_cstr<'a>(&self, source: &'a CStr) -> impl Iterator<Item = Token<'a>> {
        self.0.tokenize_cstr(source).map(Token)
    }
}

#[cfg(feature = "sqlite")]
impl Default for Tokenizer {
    fn default() -> Self {
        Self::new()
    }
}

/// An active cursor over `SQLite` tokens. Produced by [`Tokenizer::tokenize`].
///
/// Iterates [`Token`]s with SQLite-specific token types.
#[cfg(feature = "sqlite")]
pub struct TokenCursor<'a>(TypedTokenCursor<'a, SqliteGrammar>);

#[cfg(feature = "sqlite")]
impl<'a> Iterator for TokenCursor<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(Token)
    }
}

/// A `SQLite` token: token type + source text slice. Produced by [`TokenCursor`].
#[cfg(feature = "sqlite")]
pub struct Token<'a>(TypedToken<'a, SqliteGrammar>);

#[cfg(feature = "sqlite")]
impl<'a> Token<'a> {
    /// The `SQLite` token type.
    pub fn token_type(&self) -> TokenType {
        self.0.token_type
    }

    /// The source text slice covered by this token.
    pub fn text(&self) -> &'a str {
        self.0.text
    }
}

/// A type-safe tokenizer scoped to a specific dialect `N`.
///
/// Yields [`TypedToken<'_, N>`] with `N::Token` instead of a raw `u32`.
///
/// For the common `SQLite` case use [`Tokenizer`]. For dialect-agnostic use with
/// raw `u32` ordinals use [`AnyTokenizer`].
///
/// Uses an interior-mutability checkout pattern: `tokenize()` checks out the
/// C tokenizer state at runtime, and the returned [`TypedTokenCursor`] returns
/// it on drop. This allows `tokenize()` to take `&self` rather than `&mut self`.
pub struct TypedTokenizer<G: TypedGrammar> {
    inner: Rc<RefCell<Option<TokenizerInner>>>,
    _marker: PhantomData<G>,
}

impl<G: TypedGrammar> TypedTokenizer<G> {
    /// Create a tokenizer bound to the given dialect grammar.
    ///
    /// # Panics
    ///
    /// Panics if the underlying C tokenizer allocation fails (OOM).
    pub fn new(mut grammar: G) -> Self {
        // SAFETY: create(NULL, grammar.inner) allocates a new tokenizer with
        // default malloc/free. The C side copies the grammar.
        let raw =
            NonNull::new(unsafe { ffi::CTokenizer::create(std::ptr::null(), grammar.raw().inner) })
                .expect("tokenizer allocation failed");

        TypedTokenizer {
            inner: Rc::new(RefCell::new(Some(TokenizerInner {
                raw,
                source_buf: Vec::new(),
            }))),
            _marker: PhantomData,
        }
    }

    /// Bind source text and return a [`TypedTokenCursor`] for iterating tokens.
    ///
    /// Copies the source into an internal buffer to add a null terminator
    /// (required by the C tokenizer). The cursor owns the copy, so the
    /// original `source` does not need to outlive the cursor. For zero-copy
    /// tokenization use [`tokenize_cstr`](Self::tokenize_cstr).
    ///
    /// # Panics
    ///
    /// Panics if a cursor from a previous `tokenize()` call is still alive.
    pub fn tokenize<'a>(&self, source: &'a str) -> TypedTokenCursor<'a, G> {
        let mut inner = self
            .inner
            .borrow_mut()
            .take()
            .expect("TypedTokenizer::tokenize called while a cursor is still active");

        inner.source_buf.clear();
        inner.source_buf.reserve(source.len() + 1);
        inner.source_buf.extend_from_slice(source.as_bytes());
        inner.source_buf.push(0);

        // source_buf has at least one byte (the null terminator just pushed).
        let c_source_ptr =
            NonNull::new(inner.source_buf.as_mut_ptr()).expect("source_buf is non-empty");
        // SAFETY: inner.raw is valid; c_source_ptr points to source_buf which
        // is null-terminated. source_buf lives inside inner which will be owned
        // by the cursor.
        unsafe {
            inner.raw.as_mut().reset(
                c_source_ptr.as_ptr() as *const _,
                #[allow(clippy::cast_possible_truncation)]
                {
                    source.len() as u32
                },
            );
        }
        TypedTokenCursor {
            raw: inner.raw,
            source,
            c_source_base: c_source_ptr,
            inner: Some(inner),
            slot: Rc::clone(&self.inner),
            _marker: PhantomData,
        }
    }

    /// Zero-copy variant: bind a null-terminated source and return a
    /// [`TypedTokenCursor`].
    ///
    /// The `&CStr` already guarantees a trailing `\0`, so no copy is needed.
    /// The source must be valid UTF-8 (panics otherwise).
    ///
    /// # Panics
    ///
    /// Panics if a cursor from a previous `tokenize()` call is still alive,
    /// or if `source` is not valid UTF-8.
    pub fn tokenize_cstr<'a>(&self, source: &'a CStr) -> TypedTokenCursor<'a, G> {
        let mut inner = self
            .inner
            .borrow_mut()
            .take()
            .expect("TypedTokenizer::tokenize_cstr called while a cursor is still active");

        let bytes = source.to_bytes();
        let source_str = std::str::from_utf8(bytes).expect("source must be valid UTF-8");

        // SAFETY: inner.raw is valid; source is a CStr (null-terminated, valid for 'a).
        unsafe {
            inner.raw.as_mut().reset(
                source.as_ptr(),
                #[allow(clippy::cast_possible_truncation)]
                {
                    bytes.len() as u32
                },
            );
        };
        TypedTokenCursor {
            raw: inner.raw,
            source: source_str,
            c_source_base: NonNull::new(source.as_ptr() as *mut u8).expect("CStr is non-null"),
            inner: Some(inner),
            slot: Rc::clone(&self.inner),
            _marker: PhantomData,
        }
    }
}

impl TypedTokenizer<AnyDialect> {
    /// Create a type-erased tokenizer from a [`AnyGrammar`].
    pub fn from_raw_grammar(grammar: AnyGrammar) -> Self {
        Self::new(AnyDialect { raw: grammar })
    }
}

/// A typed token: dialect token type + source text slice.
#[derive(Debug, Clone, Copy)]
pub struct TypedToken<'a, G: TypedGrammar> {
    /// Dialect-typed token variant.
    pub token_type: G::Token,
    /// Slice of the source text covered by this token.
    pub text: &'a str,
}

/// An active cursor over typed tokens from a [`TypedTokenizer`].
///
/// Tokens whose ordinal does not map to a known `G::Token` variant are silently
/// skipped; use [`AnyTokenizer`] / [`AnyTokenCursor`] to observe every ordinal.
///
/// On drop, the checked-out tokenizer state is returned to the parent
/// [`TypedTokenizer`].
pub struct TypedTokenCursor<'a, G: TypedGrammar> {
    raw: NonNull<ffi::CTokenizer>,
    source: &'a str,
    /// Base pointer of the C source buffer. Used to compute byte offsets back
    /// into the Rust `source` slice.
    c_source_base: NonNull<u8>,
    /// Checked-out tokenizer state. Returned to `slot` on drop.
    inner: Option<TokenizerInner>,
    /// Slot to return `inner` to when this cursor is dropped.
    slot: Rc<RefCell<Option<TokenizerInner>>>,
    _marker: PhantomData<G>,
}

impl<G: TypedGrammar> Drop for TypedTokenCursor<'_, G> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            *self.slot.borrow_mut() = Some(inner);
        }
    }
}

impl<'a, G: TypedGrammar> Iterator for TypedTokenCursor<'a, G> {
    type Item = TypedToken<'a, G>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut token = ffi::CToken {
                text: std::ptr::null(),
                length: 0,
                type_: 0,
            };
            // SAFETY: self.raw is valid (owned by TokenizerInner in self.inner);
            // &mut token is a valid output parameter.
            let rc = unsafe { self.raw.as_mut().next(&raw mut token) };
            if rc == 0 {
                return None;
            }

            if let Some(token_type) = G::Token::from_token_type(token.type_) {
                // Compute offset into the source string from the C pointer.
                let offset = token.text as usize - self.c_source_base.as_ptr() as usize;
                let text = &self.source[offset..offset + token.length as usize];
                return Some(TypedToken { token_type, text });
            }
        }
    }
}

/// A type-erased tokenizer. Yields [`AnyToken`]s with raw `u32` token type
/// ordinals, suitable for use across multiple dialects.
///
/// This is a type alias for [`TypedTokenizer<AnyDialect>`]. Use
/// [`AnyTokenizer::from_raw_grammar`] to construct from a [`AnyGrammar`].
pub type AnyTokenizer = TypedTokenizer<AnyDialect>;

/// A raw token: `u32` token type ordinal + source text slice.
pub type AnyToken<'a> = TypedToken<'a, AnyDialect>;

/// An active cursor over raw tokens from a [`AnyTokenizer`].
pub type AnyTokenCursor<'a> = TypedTokenCursor<'a, AnyDialect>;

// ── Crate-internal ───────────────────────────────────────────────────────────

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
        unsafe { ffi::CTokenizer::destroy(self.raw.as_ptr()) }
    }
}

// ── ffi ───────────────────────────────────────────────────────────────────────

mod ffi {
    use std::ffi::{c_char, c_int};

    /// Opaque C tokenizer type.
    pub(crate) enum CTokenizer {}

    impl CTokenizer {
        pub(crate) unsafe fn create(
            mem: *const std::ffi::c_void,
            grammar: crate::grammar::ffi::CGrammar,
        ) -> *mut Self {
            // SAFETY: caller guarantees `mem` is null or a valid mem-methods
            // pointer; `grammar` is a valid grammar descriptor.
            unsafe { syntaqlite_tokenizer_create(mem, grammar) }
        }

        pub(crate) unsafe fn reset(&mut self, source: *const c_char, len: u32) {
            // SAFETY: caller guarantees `self` is valid and `source` points to
            // at least `len` bytes of valid, null-terminated C string data.
            unsafe { syntaqlite_tokenizer_reset(self, source, len) }
        }

        pub(crate) unsafe fn next(&mut self, out: *mut CToken) -> c_int {
            // SAFETY: caller guarantees `self` is valid after a `reset` call
            // and `out` is a valid writable pointer to a `CToken`.
            unsafe { syntaqlite_tokenizer_next(self, out) }
        }

        pub(crate) unsafe fn destroy(this: *mut Self) {
            // SAFETY: caller guarantees `this` was allocated by `create` and
            // has not been freed yet (called exactly once from `Drop`).
            unsafe { syntaqlite_tokenizer_destroy(this) }
        }
    }

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
        fn syntaqlite_tokenizer_create(
            mem: *const std::ffi::c_void,
            grammar: crate::grammar::ffi::CGrammar,
        ) -> *mut CTokenizer;
        fn syntaqlite_tokenizer_reset(tok: *mut CTokenizer, source: *const c_char, len: u32);
        fn syntaqlite_tokenizer_next(tok: *mut CTokenizer, out: *mut CToken) -> c_int;
        fn syntaqlite_tokenizer_destroy(tok: *mut CTokenizer);
    }
}

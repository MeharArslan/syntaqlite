// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::RefCell;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::rc::Rc;

use crate::any::AnyTokenType;
use crate::ast::GrammarTokenType;
use crate::grammar::{AnyGrammar, TypedGrammar};

#[cfg(feature = "sqlite")]
use crate::sqlite::grammar::Grammar;
#[cfg(feature = "sqlite")]
use crate::sqlite::tokens::TokenType;

// ── Public API ───────────────────────────────────────────────────────────────

/// A tokenizer for the `SQLite` dialect.
///
/// Yields [`TypedToken`]s with SQLite-specific token types. For other dialects
/// use [`TypedTokenizer`] directly; for grammar-agnostic use with raw `u32`
/// ordinals use [`AnyTokenizer`].
#[cfg(feature = "sqlite")]
#[doc(hidden)]
pub struct Tokenizer(TypedTokenizer<Grammar>);

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

/// A `SQLite` token: token type + source text slice. Produced by [`Tokenizer::tokenize`].
#[cfg(feature = "sqlite")]
#[doc(hidden)]
pub struct Token<'a>(TypedToken<'a, Grammar>);

#[cfg(feature = "sqlite")]
impl<'a> Token<'a> {
    /// The `SQLite` token type.
    pub fn token_type(&self) -> TokenType {
        self.0.token_type()
    }

    /// The source text slice covered by this token.
    pub fn text(&self) -> &'a str {
        self.0.text()
    }
}

/// A type-safe tokenizer scoped to a specific dialect `N`.
///
/// Yields [`TypedToken<'_, N>`] with `N::Token` instead of a raw `u32`.
///
/// For the common `SQLite` case use [`Tokenizer`]. For grammar-agnostic use with
/// raw `u32` ordinals use [`AnyTokenizer`].
///
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
    pub fn new(grammar: G) -> Self {
        // SAFETY: create(NULL, grammar.inner) allocates a new tokenizer with
        // default malloc/free. The C side copies the grammar.
        let raw = NonNull::new(unsafe {
            ffi::CTokenizer::create(std::ptr::null(), Into::<AnyGrammar>::into(grammar).inner)
        })
        .expect("tokenizer allocation failed");

        TypedTokenizer {
            inner: Rc::new(RefCell::new(Some(TokenizerInner {
                raw,
                source_buf: Vec::new(),
            }))),
            _marker: PhantomData,
        }
    }

    /// Bind source text and return an iterator over typed tokens.
    ///
    /// The source is copied; the original does not need to outlive the iterator.
    /// For zero-copy tokenization use [`tokenize_cstr`](Self::tokenize_cstr).
    ///
    /// # Panics
    ///
    /// Panics if an iterator from a previous `tokenize()` call is still alive.
    pub fn tokenize<'a>(
        &self,
        source: &'a str,
    ) -> impl Iterator<Item = TypedToken<'a, G>> + use<'a, G> {
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

    /// Zero-copy variant: bind a null-terminated source and return an iterator over typed tokens.
    ///
    /// No copy is performed. The source must be valid UTF-8 (panics otherwise).
    ///
    /// # Panics
    ///
    /// Panics if an iterator from a previous `tokenize()` call is still alive,
    /// or if `source` is not valid UTF-8.
    pub fn tokenize_cstr<'a>(
        &self,
        source: &'a CStr,
    ) -> impl Iterator<Item = TypedToken<'a, G>> + use<'a, G> {
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

/// A typed token: dialect token type + source text slice.
#[derive(Debug, Clone, Copy)]
pub struct TypedToken<'a, G: TypedGrammar> {
    token_type: G::Token,
    text: &'a str,
}

impl<'a, G: TypedGrammar> TypedToken<'a, G> {
    /// The dialect-typed token variant.
    pub fn token_type(&self) -> G::Token {
        self.token_type
    }

    /// The source text slice covered by this token.
    pub fn text(&self) -> &'a str {
        self.text
    }
}

/// A type-erased tokenizer. Yields [`AnyToken`]s whose token type is
/// [`AnyTokenType`](crate::any::AnyTokenType), suitable for use across multiple dialects.
///
/// This is a type alias for [`TypedTokenizer<AnyGrammar>`].
pub type AnyTokenizer = TypedTokenizer<AnyGrammar>;

/// A type-erased token: [`AnyTokenType`](crate::any::AnyTokenType) + source text slice.
pub type AnyToken<'a> = TypedToken<'a, AnyGrammar>;

// ── Crate-internal ───────────────────────────────────────────────────────────

/// An iterator over tokens produced by [`TypedTokenizer::tokenize`] or [`TypedTokenizer::tokenize_cstr`].
///
/// Returned by the `tokenize` family of methods on [`TypedTokenizer`] and [`AnyTokenizer`].
/// Implements [`Iterator`]`<Item = `[`TypedToken`]`<'a, G>>`.
struct TypedTokenCursor<'a, G: TypedGrammar> {
    raw: NonNull<ffi::CTokenizer>,
    source: &'a str,
    /// Base pointer of the C source buffer. Used to compute byte offsets back
    /// into the Rust `source` slice.
    c_source_base: NonNull<u8>,
    inner: Option<TokenizerInner>,
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

            if let Some(token_type) = G::Token::from_token_type(AnyTokenType(token.type_)) {
                // Compute offset into the source string from the C pointer.
                let offset = token.text as usize - self.c_source_base.as_ptr() as usize;
                let text = &self.source[offset..offset + token.length as usize];
                return Some(TypedToken { token_type, text });
            }
        }
    }
}

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
    use std::ffi::c_char;

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

        pub(crate) unsafe fn next(&mut self, out: *mut CToken) -> u32 {
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
        fn syntaqlite_tokenizer_next(tok: *mut CTokenizer, out: *mut CToken) -> u32;
        fn syntaqlite_tokenizer_destroy(tok: *mut CTokenizer);
    }
}

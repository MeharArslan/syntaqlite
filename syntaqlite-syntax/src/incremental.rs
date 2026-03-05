// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::Range;
use std::ptr::NonNull;
use std::rc::Rc;

use crate::ast::{AnyDialect, AnyNode, AnyNodeId};
use crate::grammar::{AnyGrammar, TypedGrammar};
use crate::parser::{
    AnyStatementResult, CParser, Comment, MacroRegion, ParserInner, TokenPos, TypedParseError,
    TypedStatementResult,
};

// ── Public API ───────────────────────────────────────────────────────────────

/// A type-safe incremental parse session for a specific dialect `G`.
///
/// Feed tokens one at a time via `feed_token` and signal
/// end of input with `finish`.
///
/// For the `SQLite` dialect use [`IncrementalParseSession`]. For dialect-agnostic
/// use with raw grammars use [`AnyIncrementalParseSession`].
///
/// Obtained via `TypedParser::incremental_parse`.
pub struct TypedIncrementalParseSession<G: TypedGrammar> {
    /// Base pointer into the internal source buffer. `feed_token` uses this
    /// to compute the C-side token pointer from byte-offset spans.
    #[allow(dead_code)]
    c_source_ptr: NonNull<u8>,
    #[allow(dead_code)]
    grammar: AnyGrammar,
    /// Checked-out parser state. Returned to `slot` on drop.
    inner: Option<ParserInner>,
    /// Slot to return `inner` to when this session is dropped.
    slot: Rc<RefCell<Option<ParserInner>>>,
    finished: bool,
    _marker: PhantomData<G>,
}

impl<G: TypedGrammar> Drop for TypedIncrementalParseSession<G> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            *self.slot.borrow_mut() = Some(inner);
        }
    }
}

#[allow(dead_code)]
impl<G: TypedGrammar> TypedIncrementalParseSession<G> {
    pub(crate) fn new(
        c_source_ptr: NonNull<u8>,
        grammar: AnyGrammar,
        inner: ParserInner,
        slot: Rc<RefCell<Option<ParserInner>>>,
    ) -> Self {
        TypedIncrementalParseSession {
            c_source_ptr,
            grammar,
            inner: Some(inner),
            slot,
            finished: false,
            _marker: PhantomData,
        }
    }

    fn assert_not_finished(&self) {
        assert!(
            !self.finished,
            "TypedIncrementalParseSession used after finish()"
        );
    }

    fn raw_ptr(&self) -> *mut CParser {
        self.inner
            .as_ref()
            .expect("inner taken after finish()")
            .raw
            .as_ptr()
    }

    /// Build a typed [`TypedStatementResult`] for the current parser arena.
    fn typed_stmt_result(&self) -> TypedStatementResult<'_, G> {
        let inner = self.inner.as_ref().expect("inner taken after finish()");
        let source_len = inner.source_buf.len().saturating_sub(1);
        // SAFETY: source_buf was populated from valid UTF-8 (&str) in
        // reset_parser. The first source_len bytes are the original source.
        let source = unsafe { std::str::from_utf8_unchecked(&inner.source_buf[..source_len]) };
        // SAFETY: inner.raw is valid (owned via ParserInner, not yet destroyed).
        unsafe { TypedStatementResult::new(inner.raw.as_ptr(), source, self.grammar) }
    }

    /// Map a C return code to a typed statement result.
    ///
    /// - `0` → `None` (keep going, no statement yet)
    /// - `1` → `Some(Ok(result))` (statement parsed cleanly)
    /// - `2` or `-1` → `Some(Err(err))` (error recovery / unrecoverable)
    fn result_from_rc(
        &self,
        rc: i32,
    ) -> Option<Result<TypedStatementResult<'_, G>, TypedParseError<'_, G>>> {
        if rc == 0 {
            return None;
        }
        let result = self.typed_stmt_result();
        if rc == 1 {
            Some(Ok(result))
        } else {
            Some(Err(TypedParseError::new(result)))
        }
    }

    /// Feed a single token to the parser.
    ///
    /// `TK_SPACE` is silently skipped. `TK_COMMENT` is recorded as a comment
    /// (when `collect_tokens` is enabled) but not fed to the parser.
    ///
    /// Returns:
    /// - `None` — keep going, statement not yet complete.
    /// - `Some(Ok(result))` — statement parsed cleanly; use
    ///   [`TypedStatementResult::root`] to access the typed AST.
    /// - `Some(Err(err))` — parse error; `err.root()` may contain a partial
    ///   recovery tree.
    ///
    /// `span` is a byte range into the source text bound by this session.
    /// `token_type` is a raw token type ordinal (dialect-specific).
    pub(crate) fn feed_token(
        &mut self,
        token_type: u32,
        span: Range<usize>,
    ) -> Option<Result<TypedStatementResult<'_, G>, TypedParseError<'_, G>>> {
        self.assert_not_finished();
        // SAFETY: c_source_ptr is valid for the source length; raw is valid.
        let rc = unsafe {
            let c_text = self.c_source_ptr.as_ptr().add(span.start);
            #[allow(clippy::cast_possible_truncation)]
            (*self.raw_ptr()).feed_token(token_type, c_text as *const _, span.len() as u32)
        };
        self.result_from_rc(rc)
    }

    /// Signal end of input.
    ///
    /// Synthesizes a SEMI if the last token wasn't one, then sends EOF to the
    /// parser. Returns:
    /// - `None` — nothing was pending (empty input or bare semicolons only).
    /// - `Some(Ok(result))` — final statement parsed cleanly.
    /// - `Some(Err(err))` — parse error; `err.root()` may contain a partial
    ///   recovery tree.
    ///
    /// No further methods may be called after `finish()`.
    pub(crate) fn finish(
        &mut self,
    ) -> Option<Result<TypedStatementResult<'_, G>, TypedParseError<'_, G>>> {
        self.assert_not_finished();
        self.finished = true;
        // SAFETY: raw is valid.
        let rc = unsafe { (*self.raw_ptr()).finish() };
        self.result_from_rc(rc)
    }

    /// Return terminal token IDs that are valid lookaheads at the current
    /// parser state.
    ///
    /// Useful for completion engines after feeding tokens up to the session.
    /// Returns raw dialect-specific token ordinals.
    pub fn expected_tokens(&self) -> Vec<u32> {
        self.assert_not_finished();
        // Use a stack buffer to avoid the count-then-allocate double FFI call.
        // 256 covers virtually all parser states; fall back to heap for outliers.
        let raw = self.raw_ptr();
        let mut stack_buf = [0u32; 256];
        // SAFETY: raw is valid and exclusively borrowed via &self; stack_buf is
        // a valid output buffer.
        #[allow(clippy::cast_possible_truncation)]
        let total =
            unsafe { (*raw).expected_tokens(stack_buf.as_mut_ptr(), stack_buf.len() as u32) };
        if total == 0 {
            return Vec::new();
        }

        let count = total as usize;
        if count <= stack_buf.len() {
            stack_buf[..count].to_vec()
        } else {
            // Rare: more tokens than stack buffer. Heap-allocate and re-query.
            let mut heap_buf = vec![0u32; count];
            // SAFETY: raw is valid; heap_buf is sized to hold `total` entries.
            let written = unsafe { (*raw).expected_tokens(heap_buf.as_mut_ptr(), total) };
            let len = written.clamp(0, total) as usize;
            heap_buf.truncate(len);
            heap_buf.into_iter().collect()
        }
    }

    /// Return the semantic completion context at the current parser state.
    ///
    /// Returns a raw `u32`: `0` = Unknown, `1` = Expression, `2` = `TableRef`.
    pub fn completion_context(&self) -> u32 {
        self.assert_not_finished();
        // SAFETY: raw is valid and exclusively borrowed via &self.
        unsafe { (*self.raw_ptr()).completion_context() }
    }

    /// Return the number of nodes currently in the parser arena.
    pub fn node_count(&self) -> u32 {
        // SAFETY: raw is valid and exclusively borrowed via &self.
        unsafe { (*self.raw_ptr()).node_count() }
    }

    /// Mark subsequent fed tokens as being inside a macro expansion.
    ///
    /// `call_offset` and `call_length` describe the macro call's byte range
    /// in the original source. Calls may nest (for nested macro expansions).
    pub fn begin_macro(&mut self, call_offset: u32, call_length: u32) {
        self.assert_not_finished();
        // SAFETY: raw is valid and exclusively borrowed via &mut self.
        unsafe { (*self.raw_ptr()).begin_macro(call_offset, call_length) }
    }

    /// End the innermost macro expansion region.
    pub fn end_macro(&mut self) {
        self.assert_not_finished();
        // SAFETY: raw is valid and exclusively borrowed via &mut self.
        unsafe { (*self.raw_ptr()).end_macro() }
    }

    /// Build a type-erased [`AnyStatementResult`] for the parser arena.
    ///
    /// Lightweight (no allocation) — packages the raw parser pointer with a
    /// `&str` view of the owned source buffer.
    pub(crate) fn stmt_result(&self) -> AnyStatementResult<'_> {
        self.typed_stmt_result().erase()
    }

    /// Wrap an [`AnyNodeId`] as an [`AnyNode`] using this session's `stmt_result`.
    pub(crate) fn node_ref(&self, id: AnyNodeId) -> AnyNode<'_> {
        AnyNode {
            id,
            stmt_result: self.stmt_result(),
        }
    }

    /// Return all comments captured during parsing.
    pub(crate) fn comments(&self) -> &[Comment] {
        self.stmt_result().comments()
    }

    /// Return all token positions collected during parsing.
    ///
    /// Only populated when the parser was built with `collect_tokens: true`.
    pub(crate) fn tokens(&self) -> &[TokenPos] {
        self.stmt_result().tokens()
    }

    /// Return all macro regions recorded via [`begin_macro`](Self::begin_macro)
    /// / [`end_macro`](Self::end_macro).
    pub(crate) fn macro_regions(&self) -> &[MacroRegion] {
        // SAFETY: raw is valid (owned via ParserInner, valid for &self).
        unsafe { (*self.raw_ptr()).result_macros() }
    }
}

/// A type-erased incremental parse session.
/// Yields type-erased statement results with raw node types, suitable for use across multiple dialects.
pub type AnyIncrementalParseSession = TypedIncrementalParseSession<AnyDialect>;

/// An incremental parse session for the `SQLite` dialect. Produced by
/// [`Parser::incremental_parse`](crate::parser::Parser::incremental_parse).
///
/// Feed tokens one at a time via `feed_token` and signal
/// end of input with `finish`.
///
/// On drop, the checked-out parser state is returned to the parent
/// [`Parser`](crate::parser::Parser).
#[cfg(feature = "sqlite")]
pub struct IncrementalParseSession(
    TypedIncrementalParseSession<crate::sqlite::grammar::SqliteGrammar>,
);

#[cfg(feature = "sqlite")]
#[allow(dead_code)]
impl IncrementalParseSession {
    /// Feed a single token to the parser.
    ///
    /// Returns:
    /// - `None` — keep going, statement not yet complete.
    /// - `Some(Ok(result))` — statement parsed cleanly.
    /// - `Some(Err(e))` — parse error; `e.root()` may contain a partial
    ///   recovery tree.
    pub(crate) fn feed_token(
        &mut self,
        token_type: u32,
        span: Range<usize>,
    ) -> Option<Result<crate::parser::StatementResult<'_>, crate::parser::ParseError<'_>>> {
        Some(match self.0.feed_token(token_type, span)? {
            Ok(result) => Ok(crate::parser::StatementResult(result)),
            Err(err) => Err(crate::parser::ParseError(err)),
        })
    }

    /// Signal end of input.
    ///
    /// Returns:
    /// - `None` — nothing was pending.
    /// - `Some(Ok(result))` — final statement parsed cleanly.
    /// - `Some(Err(e))` — parse error; `e.root()` may contain a partial
    ///   recovery tree.
    ///
    /// No further methods may be called after `finish()`.
    pub(crate) fn finish(
        &mut self,
    ) -> Option<Result<crate::parser::StatementResult<'_>, crate::parser::ParseError<'_>>> {
        Some(match self.0.finish()? {
            Ok(result) => Ok(crate::parser::StatementResult(result)),
            Err(err) => Err(crate::parser::ParseError(err)),
        })
    }

    /// Return terminal token IDs that are valid lookaheads at the current
    /// parser state.
    pub fn expected_tokens(&self) -> Vec<u32> {
        self.0.expected_tokens()
    }

    /// Return the semantic completion context at the current parser state.
    ///
    /// Returns a raw `u32`: `0` = Unknown, `1` = Expression, `2` = `TableRef`.
    pub fn completion_context(&self) -> u32 {
        self.0.completion_context()
    }

    /// Return the number of nodes currently in the parser arena.
    pub fn node_count(&self) -> u32 {
        self.0.node_count()
    }

    /// Mark subsequent fed tokens as being inside a macro expansion.
    pub fn begin_macro(&mut self, call_offset: u32, call_length: u32) {
        self.0.begin_macro(call_offset, call_length);
    }

    /// End the innermost macro expansion region.
    pub fn end_macro(&mut self) {
        self.0.end_macro();
    }

    pub(crate) fn stmt_result(&self) -> AnyStatementResult<'_> {
        self.0.stmt_result()
    }

    pub(crate) fn node_ref(&self, id: AnyNodeId) -> AnyNode<'_> {
        self.0.node_ref(id)
    }

    pub(crate) fn comments(&self) -> &[Comment] {
        self.0.comments()
    }

    pub(crate) fn tokens(&self) -> &[TokenPos] {
        self.0.tokens()
    }

    pub(crate) fn macro_regions(&self) -> &[MacroRegion] {
        self.0.macro_regions()
    }
}

#[cfg(feature = "sqlite")]
impl From<TypedIncrementalParseSession<crate::sqlite::grammar::SqliteGrammar>>
    for IncrementalParseSession
{
    fn from(inner: TypedIncrementalParseSession<crate::sqlite::grammar::SqliteGrammar>) -> Self {
        IncrementalParseSession(inner)
    }
}

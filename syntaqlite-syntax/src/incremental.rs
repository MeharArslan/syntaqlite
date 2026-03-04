// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::RefCell;
use std::ffi::{CStr, c_int};
use std::ops::Range;
use std::ptr::NonNull;
use std::rc::Rc;

use crate::ast::{AnyNode, AnyNodeId};
use crate::grammar::AnyGrammar;
use crate::parser::{AnyParseError, AnyStatementResult, CParser, ParserInner, TypedStatementResult, ffi_slice};

// ── Public API ───────────────────────────────────────────────────────────────

/// A low-level cursor for feeding tokens one at a time.
///
/// Obtained via [`Parser::incremental_parse`](crate::parser::Parser::incremental_parse).
/// After calling [`finish()`](Self::finish), no further methods may be called.
///
/// On drop, the checked-out parser state is returned to the parent [`Parser`](crate::parser::Parser).
pub struct IncrementalCursor {
    /// Base pointer into the internal source buffer. `feed_token` uses this
    /// to compute the C-side token pointer from byte-offset spans.
    c_source_ptr: NonNull<u8>,
    grammar: AnyGrammar,
    /// Checked-out parser state. Returned to `slot` on drop.
    inner: Option<ParserInner>,
    /// Slot to return `inner` to when this cursor is dropped.
    slot: Rc<RefCell<Option<ParserInner>>>,
    finished: bool,
}

impl Drop for IncrementalCursor {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            *self.slot.borrow_mut() = Some(inner);
        }
    }
}

impl IncrementalCursor {
    pub(crate) fn new(
        c_source_ptr: NonNull<u8>,
        grammar: AnyGrammar,
        inner: ParserInner,
        slot: Rc<RefCell<Option<ParserInner>>>,
    ) -> Self {
        IncrementalCursor {
            c_source_ptr,
            grammar,
            inner: Some(inner),
            slot,
            finished: false,
        }
    }

    fn assert_not_finished(&self) {
        assert!(!self.finished, "IncrementalCursor used after finish()");
    }

    fn raw_ptr(&self) -> *mut CParser {
        self.inner.as_ref().unwrap().raw.as_ptr()
    }

    /// Read the parser result after a statement-completing or error return code.
    ///
    /// Return codes from C (`SYNTAQLITE_PARSE_*`):
    /// - `1` = clean success
    /// - `2` = success with error recovery (tree has `CErrorNode` holes)
    /// - `-1` = unrecoverable error
    fn parse_result(&self, rc: c_int) -> Result<AnyNodeId, AnyParseError> {
        // SAFETY: raw is valid; result accessors read directly from parser
        // fields valid for the parser lifetime.
        unsafe {
            let raw = self.raw_ptr();
            let root_id = AnyNodeId(ffi::syntaqlite_result_root(raw));

            if rc == 1 {
                return Ok(root_id);
            }

            // rc == 2 or rc == -1: build error from accessors.
            let msg_ptr = ffi::syntaqlite_result_error_msg(raw);
            let msg = if msg_ptr.is_null() {
                "parse error".to_string()
            } else {
                // SAFETY: error_msg is a NUL-terminated string in the parser's buffer.
                CStr::from_ptr(msg_ptr).to_string_lossy().into_owned()
            };
            let error_offset = ffi::syntaqlite_result_error_offset(raw);
            let error_length = ffi::syntaqlite_result_error_length(raw);
            let offset = if error_offset == 0xFFFF_FFFF {
                None
            } else {
                Some(error_offset as usize)
            };
            let length = if error_length == 0 {
                None
            } else {
                Some(error_length as usize)
            };

            if rc == 2 {
                // Error recovery succeeded — tree is valid but has CErrorNode holes.
                return Err(AnyParseError {
                    message: msg,
                    offset,
                    length,
                    root: Some(root_id),
                });
            }

            // rc == -1: unrecoverable error, root may be NULL.
            let root = if !root_id.is_null() {
                Some(root_id)
            } else {
                None
            };
            Err(AnyParseError {
                message: msg,
                offset,
                length,
                root,
            })
        }
    }

    /// Feed a single token to the parser.
    ///
    /// `TK_SPACE` is silently skipped. `TK_COMMENT` is recorded as a comment
    /// (when `collect_tokens` is enabled) but not fed to the parser.
    ///
    /// Returns `Ok(Some(root_id))` when a statement completes cleanly,
    /// `Ok(None)` to keep going, or `Err` on parse error. When error
    /// recovery succeeds, the error's `root` field contains the partial tree.
    ///
    /// `span` is a byte range into the source text bound by this cursor.
    /// `token_type` is a raw token type ordinal (dialect-specific).
    pub(crate) fn feed_token(
        &mut self,
        token_type: u32,
        span: Range<usize>,
    ) -> Result<Option<AnyNodeId>, AnyParseError> {
        self.assert_not_finished();
        // SAFETY: c_source_ptr is valid for the source length; raw is valid.
        let rc = unsafe {
            let c_text = self.c_source_ptr.as_ptr().add(span.start);
            ffi::syntaqlite_parser_feed_token(
                self.raw_ptr(),
                token_type as c_int,
                c_text as *const _,
                span.len() as c_int,
            )
        };
        match rc {
            0 => Ok(None),
            _ => self.parse_result(rc).map(Some),
        }
    }

    /// Signal end of input.
    ///
    /// Synthesizes a SEMI if the last token wasn't one, then sends EOF to
    /// the parser. Returns `Ok(Some(root_id))` if a final statement completed
    /// cleanly, `Ok(None)` if there was nothing pending, or `Err` on parse
    /// error. When error recovery succeeds, the error's `root` field contains
    /// the partial tree.
    ///
    /// No further methods may be called after `finish()`.
    pub(crate) fn finish(&mut self) -> Result<Option<AnyNodeId>, AnyParseError> {
        self.assert_not_finished();
        self.finished = true;
        // SAFETY: raw is valid.
        let rc = unsafe { ffi::syntaqlite_parser_finish(self.raw_ptr()) };
        match rc {
            0 => Ok(None),
            _ => self.parse_result(rc).map(Some),
        }
    }

    /// Return terminal token IDs that are valid lookaheads at the current
    /// parser state.
    ///
    /// Useful for completion engines after feeding tokens up to the cursor.
    /// Returns raw dialect-specific token ordinals.
    pub fn expected_tokens(&self) -> Vec<u32> {
        self.assert_not_finished();
        // Use a stack buffer to avoid the count-then-allocate double FFI call.
        // 256 covers virtually all parser states; fall back to heap for outliers.
        let raw = self.raw_ptr();
        let mut stack_buf = [0 as c_int; 256];
        // SAFETY: raw is valid and exclusively borrowed via &self; stack_buf is
        // a valid output buffer.
        let total = unsafe {
            ffi::syntaqlite_parser_expected_tokens(
                raw,
                stack_buf.as_mut_ptr(),
                stack_buf.len() as c_int,
            )
        };
        if total <= 0 {
            return Vec::new();
        }

        let count = total as usize;
        if count <= stack_buf.len() {
            stack_buf[..count].iter().map(|&t| t as u32).collect()
        } else {
            // Rare: more tokens than stack buffer. Heap-allocate and re-query.
            let mut heap_buf = vec![0 as c_int; count];
            // SAFETY: raw is valid; heap_buf is sized to hold `total` entries.
            let written = unsafe {
                ffi::syntaqlite_parser_expected_tokens(raw, heap_buf.as_mut_ptr(), total as c_int)
            };
            let len = written.clamp(0, total) as usize;
            heap_buf.truncate(len);
            heap_buf.into_iter().map(|t| t as u32).collect()
        }
    }

    /// Return the semantic completion context at the current parser state.
    ///
    /// Returns a raw `u32`: `0` = Unknown, `1` = Expression, `2` = TableRef.
    pub fn completion_context(&self) -> u32 {
        self.assert_not_finished();
        // SAFETY: raw is valid and exclusively borrowed via &self.
        unsafe { ffi::syntaqlite_parser_completion_context(self.raw_ptr()) }
    }

    /// Return the number of nodes currently in the parser arena.
    ///
    /// Flushes any pending list nodes first, so all node data is consistent.
    pub fn node_count(&self) -> u32 {
        // SAFETY: raw is valid and exclusively borrowed via &self.
        unsafe { ffi::syntaqlite_parser_node_count(self.raw_ptr()) }
    }

    /// Mark subsequent fed tokens as being inside a macro expansion.
    ///
    /// `call_offset` and `call_length` describe the macro call's byte range
    /// in the original source. Calls may nest (for nested macro expansions).
    pub fn begin_macro(&mut self, call_offset: u32, call_length: u32) {
        self.assert_not_finished();
        // SAFETY: raw is valid and exclusively borrowed via &mut self.
        unsafe {
            ffi::syntaqlite_parser_begin_macro(self.raw_ptr(), call_offset, call_length);
        }
    }

    /// End the innermost macro expansion region.
    pub fn end_macro(&mut self) {
        self.assert_not_finished();
        // SAFETY: raw is valid and exclusively borrowed via &mut self.
        unsafe {
            ffi::syntaqlite_parser_end_macro(self.raw_ptr());
        }
    }

    /// Build a [`ParseResult`] for the parser arena, borrowing source text
    /// from the internal buffer.
    ///
    /// Lightweight (no allocation) — packages the raw parser pointer with a
    /// `&str` view of the owned source buffer.
    pub(crate) fn reader(&self) -> AnyStatementResult<'_> {
        let inner = self.inner.as_ref().unwrap();
        let source_len = inner.source_buf.len().saturating_sub(1);
        // SAFETY: source_buf was populated from valid UTF-8 (&str) in
        // reset_parser. The first source_len bytes are the original source.
        let source = unsafe { std::str::from_utf8_unchecked(&inner.source_buf[..source_len]) };
        // SAFETY: inner.raw is valid (owned via ParserInner, not yet destroyed).
        unsafe { TypedStatementResult::new(inner.raw.as_ptr(), source, self.grammar) }
    }

    /// Wrap a [`AnyNodeId`] as an [`AnyNode`] using this cursor's reader.
    pub(crate) fn node_ref(&self, id: AnyNodeId) -> AnyNode<'_> {
        AnyNode {
            id,
            reader: self.reader(),
        }
    }

    /// Return all comments captured during parsing.
    pub(crate) fn comments(&self) -> &[crate::parser::Comment] {
        self.reader().comments()
    }

    /// Return all token positions collected during parsing.
    ///
    /// Only populated when the parser was built with `collect_tokens: true`.
    pub(crate) fn tokens(&self) -> &[crate::parser::TokenPos] {
        self.reader().tokens()
    }

    /// Return all macro regions recorded via [`begin_macro`](Self::begin_macro)
    /// / [`end_macro`](Self::end_macro).
    pub(crate) fn macro_regions(&self) -> &[MacroRegion] {
        // SAFETY: raw is valid (owned via ParserInner, valid for &self).
        unsafe { ffi_slice(self.raw_ptr(), ffi::syntaqlite_result_macros) }
    }
}

// ── Crate-internal ───────────────────────────────────────────────────────────

pub(crate) use ffi::CMacroRegion as MacroRegion;

// ── ffi ───────────────────────────────────────────────────────────────────────

mod ffi {
    use std::ffi::{c_char, c_int};

    /// A recorded macro invocation region, populated via `begin_macro` /
    /// `end_macro`. Used by the formatter to reconstruct macro calls.
    ///
    /// Mirrors C `SyntaqliteMacroRegion` from `include/syntaqlite/parser.h`.
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub(crate) struct CMacroRegion {
        /// Byte offset of the macro call in the original source.
        pub(crate) call_offset: u32,
        /// Byte length of the entire macro call.
        pub(crate) call_length: u32,
    }

    unsafe extern "C" {
        // Low-level token-feeding API
        pub(super) fn syntaqlite_parser_feed_token(
            p: *mut super::CParser,
            token_type: c_int,
            text: *const c_char,
            len: c_int,
        ) -> c_int;
        pub(super) fn syntaqlite_parser_finish(p: *mut super::CParser) -> c_int;
        pub(super) fn syntaqlite_parser_expected_tokens(
            p: *mut super::CParser,
            out_tokens: *mut c_int,
            out_cap: c_int,
        ) -> c_int;
        pub(super) fn syntaqlite_parser_completion_context(p: *mut super::CParser) -> u32;
        pub(super) fn syntaqlite_parser_node_count(p: *mut super::CParser) -> u32;

        // Result accessors (valid after feed_token/finish returns non-DONE)
        pub(super) fn syntaqlite_result_root(p: *mut super::CParser) -> u32;
        pub(super) fn syntaqlite_result_error_msg(p: *mut super::CParser) -> *const c_char;
        pub(super) fn syntaqlite_result_error_offset(p: *mut super::CParser) -> u32;
        pub(super) fn syntaqlite_result_error_length(p: *mut super::CParser) -> u32;
        pub(super) fn syntaqlite_result_macros(
            p: *mut super::CParser,
            count: *mut u32,
        ) -> *const CMacroRegion;

        // Macro region tracking (input-side)
        pub(super) fn syntaqlite_parser_begin_macro(
            p: *mut super::CParser,
            call_offset: u32,
            call_length: u32,
        );
        pub(super) fn syntaqlite_parser_end_macro(p: *mut super::CParser);
    }
}

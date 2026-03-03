// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::RefCell;
use std::ffi::{CStr, c_int};
use std::ops::Range;
use std::ptr::NonNull;
use std::rc::Rc;

use crate::DialectConfig;
use crate::NodeId;
use crate::NodeRef;
use crate::RawDialect;
use crate::parser::{
    syntaqlite_create_parser_with_dialect, syntaqlite_parser_begin_macro,
    syntaqlite_parser_completion_context, syntaqlite_parser_destroy, syntaqlite_parser_end_macro,
    syntaqlite_parser_expected_tokens, syntaqlite_parser_feed_token, syntaqlite_parser_finish,
    syntaqlite_parser_node_count, syntaqlite_parser_result, syntaqlite_parser_set_collect_tokens,
    syntaqlite_parser_set_dialect_config, syntaqlite_parser_set_trace,
};
use crate::raw_session::{ParserConfig, reset_parser, reset_parser_cstr};
use crate::{Comment, MacroRegion, ParseResult, Parser};
use crate::{ParseError, RawParseResult};

/// Holds the C parser handle and mutable state for incremental parsing.
/// Checked out by cursors at runtime and returned on [`Drop`].
pub(crate) struct IncrementalInner {
    raw: NonNull<Parser>,
    source_buf: Vec<u8>,
}

impl Drop for IncrementalInner {
    fn drop(&mut self) {
        // SAFETY: self.raw was allocated by syntaqlite_create_parser_with_dialect
        // and has not been freed (Drop runs exactly once).
        unsafe { syntaqlite_parser_destroy(self.raw.as_ptr()) }
    }
}

/// A low-level parser for token-by-token feeding. Owns its own C parser
/// handle and source buffer, independent of `RawParser`.
///
/// Uses the same interior-mutability checkout pattern as [`super::RawParser`].
pub struct RawIncrementalParser<'d> {
    inner: Rc<RefCell<Option<IncrementalInner>>>,
    dialect: RawDialect<'d>,
}

impl<'d> RawIncrementalParser<'d> {
    /// Create an incremental parser bound to the given dialect with default
    /// configuration (token collection enabled).
    pub fn new(dialect: impl Into<RawDialect<'d>>) -> Self {
        Self::with_config(
            dialect,
            &ParserConfig {
                collect_tokens: true,
                ..ParserConfig::default()
            },
        )
    }

    /// Create an incremental parser bound to the given dialect with custom
    /// configuration.
    pub fn with_config(dialect: impl Into<RawDialect<'d>>, config: &ParserConfig) -> Self {
        let dialect = dialect.into();
        // SAFETY: syntaqlite_create_parser_with_dialect(NULL, dialect) allocates
        // a new parser with default malloc/free. dialect.raw is valid for the call.
        let raw = NonNull::new(unsafe {
            syntaqlite_create_parser_with_dialect(std::ptr::null(), dialect.raw)
        })
        .expect("parser allocation failed");

        // SAFETY: raw is freshly created (not sealed), so these calls always succeed.
        unsafe {
            syntaqlite_parser_set_trace(raw.as_ptr(), config.trace as c_int);
            syntaqlite_parser_set_collect_tokens(raw.as_ptr(), config.collect_tokens as c_int);
        }

        if let Some(dc) = config.dialect_config {
            // SAFETY: raw is valid. The C side copies the config value during
            // this call.
            unsafe {
                syntaqlite_parser_set_dialect_config(raw.as_ptr(), &dc as *const DialectConfig);
            }
        }

        let inner = IncrementalInner {
            raw,
            source_buf: Vec::new(),
        };

        RawIncrementalParser {
            inner: Rc::new(RefCell::new(Some(inner))),
            dialect,
        }
    }

    /// Bind source text and return a `RawIncrementalCursor` for token feeding.
    ///
    /// # Panics
    ///
    /// Panics if a cursor from a previous `feed()` call is still alive.
    pub fn feed<'a>(&self, source: &'a str) -> RawIncrementalCursor<'a>
    where
        'd: 'a,
    {
        let mut inner = self
            .inner
            .borrow_mut()
            .take()
            .expect("RawIncrementalParser::feed called while a cursor is still active");
        // SAFETY: inner.raw is valid (owned via IncrementalInner); source_buf
        // lives inside inner which will be owned by the cursor.
        let (reader, c_source_ptr) =
            unsafe { reset_parser(inner.raw.as_ptr(), &mut inner.source_buf, source) };
        RawIncrementalCursor {
            reader,
            c_source_ptr,
            dialect: self.dialect,
            inner: Some(inner),
            slot: Rc::clone(&self.inner),
            finished: false,
        }
    }

    /// Zero-copy variant: bind a null-terminated source and return a `RawIncrementalCursor`.
    ///
    /// # Panics
    ///
    /// Panics if a cursor from a previous `feed()` call is still alive.
    pub fn feed_cstr<'a>(&self, source: &'a CStr) -> RawIncrementalCursor<'a>
    where
        'd: 'a,
    {
        let inner = self
            .inner
            .borrow_mut()
            .take()
            .expect("RawIncrementalParser::feed_cstr called while a cursor is still active");
        // SAFETY: inner.raw is valid (owned via IncrementalInner); source is a
        // CStr (null-terminated, valid for 'a).
        let (reader, c_source_ptr) = unsafe { reset_parser_cstr(inner.raw.as_ptr(), source) };
        RawIncrementalCursor {
            reader,
            c_source_ptr,
            dialect: self.dialect,
            inner: Some(inner),
            slot: Rc::clone(&self.inner),
            finished: false,
        }
    }
}

// ── RawIncrementalCursor ──────────────────────────────────────────────────────

/// A low-level cursor for feeding tokens one at a time.
///
/// After calling `finish()`, no further methods may be called.
///
/// On drop, the checked-out parser state is returned to the parent
/// [`RawIncrementalParser`].
pub struct RawIncrementalCursor<'a> {
    reader: RawParseResult<'a>,
    /// The pointer that the C parser uses as its source base. This may differ
    /// from `source.as_ptr()` when `feed()` copies into an internal buffer.
    /// `feed_token` translates user text pointers through this so that the C
    /// code's `tok.z - ctx->source` offset arithmetic is correct regardless
    /// of whether the copying or zero-copy path was used.
    c_source_ptr: NonNull<u8>,
    dialect: RawDialect<'a>,
    /// Checked-out parser state. Returned to `slot` on drop.
    inner: Option<IncrementalInner>,
    /// Slot to return `inner` to when this cursor is dropped.
    slot: Rc<RefCell<Option<IncrementalInner>>>,
    finished: bool,
}

impl Drop for RawIncrementalCursor<'_> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            *self.slot.borrow_mut() = Some(inner);
        }
    }
}

impl<'a> RawIncrementalCursor<'a> {
    fn assert_not_finished(&self) {
        assert!(!self.finished, "RawIncrementalCursor used after finish()");
    }

    /// Read the parser result after a statement-completing or error return code.
    ///
    /// Return codes from C:
    /// - `1` = clean success
    /// - `2` = success with error recovery (tree has ErrorNode holes)
    /// - `-1` = unrecoverable error
    fn parse_result(&self, rc: c_int) -> Result<NodeId, ParseError> {
        // SAFETY: raw is valid; result struct and error_msg pointer are valid
        // for the lifetime of the parser.
        unsafe {
            let raw = self.reader.raw();
            let result = syntaqlite_parser_result(raw);
            if rc == 1 {
                return Ok(NodeId(result.root));
            }
            let err = Self::extract_error(&result);
            if rc == 2 {
                // Error recovery succeeded — tree is valid but has ErrorNode holes.
                return Err(ParseError {
                    root: Some(NodeId(result.root)),
                    ..err
                });
            }
            // rc == -1: unrecoverable error, root may be NULL.
            let root = if result.root != u32::MAX && result.root != 0 {
                Some(NodeId(result.root))
            } else {
                None
            };
            Err(ParseError { root, ..err })
        }
    }

    /// Extract error fields from a C parse result.
    ///
    /// # Safety
    /// `result.error_msg` must be null or a valid C string.
    unsafe fn extract_error(result: &ParseResult) -> ParseError {
        let msg = if result.error_msg.is_null() {
            "parse error".to_string()
        } else {
            // SAFETY: caller guarantees error_msg is a valid C string.
            unsafe { CStr::from_ptr(result.error_msg) }
                .to_string_lossy()
                .into_owned()
        };
        let offset = if result.error_offset == 0xFFFFFFFF {
            None
        } else {
            Some(result.error_offset as usize)
        };
        let length = if result.error_length == 0 {
            None
        } else {
            Some(result.error_length as usize)
        };
        ParseError {
            message: msg,
            offset,
            length,
            root: None,
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
    pub fn feed_token(
        &mut self,
        token_type: u32,
        span: Range<usize>,
    ) -> Result<Option<NodeId>, ParseError> {
        self.assert_not_finished();
        // SAFETY: c_source_ptr is valid for the source length; raw is valid.
        let raw = self.reader.raw();
        let rc = unsafe {
            let c_text = self.c_source_ptr.as_ptr().add(span.start);
            syntaqlite_parser_feed_token(
                raw,
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
    /// Synthesizes a SEMI if the last token wasn't one, and sends EOF to
    /// the parser. Returns `Ok(Some(root_id))` if a final statement
    /// completed cleanly, `Ok(None)` if there was nothing pending, or `Err`
    /// on parse error. When error recovery succeeds, the error's `root`
    /// field contains the partial tree.
    ///
    /// No further methods may be called after `finish()`.
    pub fn finish(&mut self) -> Result<Option<NodeId>, ParseError> {
        self.assert_not_finished();
        self.finished = true;
        // SAFETY: raw is valid.
        let rc = unsafe { syntaqlite_parser_finish(self.reader.raw()) };
        match rc {
            0 => Ok(None),
            _ => self.parse_result(rc).map(Some),
        }
    }

    /// Return terminal token IDs that are valid lookaheads at the current
    /// parser state.
    ///
    /// This can be used by completion engines after feeding tokens up to the
    /// cursor. Returns raw dialect-specific token ordinals.
    pub fn expected_tokens(&self) -> Vec<u32> {
        self.assert_not_finished();
        // Use a stack buffer to avoid the count-then-allocate double FFI call.
        // 256 covers virtually all parser states; fall back to heap for outliers.
        let raw = self.reader.raw();
        let mut stack_buf = [0 as c_int; 256];
        // SAFETY: raw is valid and exclusively borrowed via &self; stack_buf is
        // a valid output buffer.
        let total = unsafe {
            syntaqlite_parser_expected_tokens(raw, stack_buf.as_mut_ptr(), stack_buf.len() as c_int)
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
                syntaqlite_parser_expected_tokens(raw, heap_buf.as_mut_ptr(), total as c_int)
            };
            let len = written.clamp(0, total) as usize;
            heap_buf.truncate(len);
            heap_buf.into_iter().map(|t| t as u32).collect()
        }
    }

    /// Return the semantic completion context at the parser's current state.
    ///
    /// Returns a raw u32: 0 = Unknown, 1 = Expression, 2 = TableRef.
    pub fn completion_context(&self) -> u32 {
        self.assert_not_finished();
        // SAFETY: raw is valid and exclusively borrowed via &self.
        unsafe { syntaqlite_parser_completion_context(self.reader.raw()) }
    }

    /// Return the number of nodes currently in the parser arena.
    ///
    /// Flushes any pending list nodes first, so all node data is consistent.
    pub fn node_count(&self) -> u32 {
        // SAFETY: raw is valid and exclusively borrowed via &self.
        unsafe { syntaqlite_parser_node_count(self.reader.raw()) }
    }

    /// Mark subsequent fed tokens as being inside a macro expansion.
    ///
    /// `call_offset` and `call_length` describe the macro call's byte range
    /// in the original source. Calls may nest (for nested macro expansions).
    pub fn begin_macro(&mut self, call_offset: u32, call_length: u32) {
        self.assert_not_finished();
        // SAFETY: raw is valid and exclusively borrowed via &mut self.
        unsafe {
            syntaqlite_parser_begin_macro(self.reader.raw(), call_offset, call_length);
        }
    }

    /// End the innermost macro expansion region.
    pub fn end_macro(&mut self) {
        self.assert_not_finished();
        // SAFETY: raw is valid and exclusively borrowed via &mut self.
        unsafe {
            syntaqlite_parser_end_macro(self.reader.raw());
        }
    }

    /// Wrap a [`NodeId`] as a [`NodeRef`] using this cursor's reader and dialect.
    pub fn node_ref(&self, id: NodeId) -> NodeRef<'a> {
        NodeRef::new(id, self.reader, self.dialect)
    }

    /// Get a reference to the embedded `NodeReader`.
    pub fn reader(&self) -> RawParseResult<'a> {
        self.reader
    }

    /// Return all comments captured during parsing.
    pub fn comments(&self) -> &[Comment] {
        self.reader.comments()
    }

    /// Return all token positions collected during parsing.
    ///
    /// Only populated when the parser was built with `collect_tokens(true)`.
    pub fn tokens(&self) -> &[crate::TokenPos] {
        self.reader.tokens()
    }

    /// Return all macro regions recorded via `begin_macro`/`end_macro`.
    pub fn macro_regions(&self) -> &[MacroRegion] {
        self.reader.macro_regions()
    }
}

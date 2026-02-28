// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::ffi::{CStr, c_int};
use std::ops::Range;

use super::ffi;
use super::nodes::NodeId;
use super::session::{CursorBase, ParseError, ParserConfig};
use crate::dialect::Dialect;

/// A low-level parser for token-by-token feeding. Owns its own C parser
/// handle and source buffer, independent of `Parser`.
pub struct LowLevelParser {
    raw: *mut ffi::Parser,
    source_buf: Vec<u8>,
}

// SAFETY: Same reasoning as Parser — the C parser is self-contained.
unsafe impl Send for LowLevelParser {}

impl LowLevelParser {
    /// Create a new low-level parser for the given dialect.
    /// Token collection is enabled by default (required for formatting).
    pub fn with_dialect(dialect: &Dialect) -> Self {
        // SAFETY: syntaqlite_create_parser_with_dialect(NULL, dialect) allocates
        // a new parser with default malloc/free. dialect.raw is valid for the call.
        let raw =
            unsafe { ffi::syntaqlite_create_parser_with_dialect(std::ptr::null(), dialect.raw) };
        assert!(!raw.is_null(), "parser allocation failed");
        // SAFETY: raw is freshly created (not sealed), so this always succeeds.
        unsafe {
            ffi::syntaqlite_parser_set_collect_tokens(raw, 1);
        }
        LowLevelParser {
            raw,
            source_buf: Vec::new(),
        }
    }

    /// Create a low-level parser with the given dialect and configuration.
    pub fn with_dialect_config(dialect: &Dialect, config: &ParserConfig) -> Self {
        let tp = Self::with_dialect(dialect);
        // SAFETY: tp.raw is freshly created (not sealed), so these calls
        // always succeed.
        unsafe {
            ffi::syntaqlite_parser_set_trace(tp.raw, config.trace as c_int);
            ffi::syntaqlite_parser_set_collect_tokens(tp.raw, config.collect_tokens as c_int);
        }
        tp
    }

    /// Create a low-level parser for the built-in SQLite dialect.
    #[cfg(feature = "sqlite")]
    pub fn new() -> Self {
        Self::with_dialect(&crate::sqlite::DIALECT)
    }

    /// Create a low-level parser for the built-in SQLite dialect with the given configuration.
    #[cfg(feature = "sqlite")]
    pub fn with_config(config: &ParserConfig) -> Self {
        Self::with_dialect_config(&crate::sqlite::DIALECT, config)
    }

    /// Bind source text and return a `LowLevelCursor` for token feeding.
    pub fn feed<'a>(&'a mut self, source: &'a str) -> LowLevelCursor<'a> {
        let base = CursorBase::new(self.raw, &mut self.source_buf, source);
        LowLevelCursor {
            base,
            finished: false,
        }
    }

    /// Zero-copy variant: bind a null-terminated source and return a `LowLevelCursor`.
    pub fn feed_cstr<'a>(&'a mut self, source: &'a CStr) -> LowLevelCursor<'a> {
        let base = CursorBase::new_cstr(self.raw, source);
        LowLevelCursor {
            base,
            finished: false,
        }
    }
}

#[cfg(feature = "sqlite")]
impl Default for LowLevelParser {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LowLevelParser {
    fn drop(&mut self) {
        // SAFETY: self.raw was allocated by syntaqlite_create_parser_with_dialect
        // and has not been freed (Drop runs exactly once).
        unsafe { ffi::syntaqlite_parser_destroy(self.raw) }
    }
}

// ── LowLevelCursor ──────────────────────────────────────────────────────

/// A low-level cursor for feeding tokens one at a time.
///
/// After calling `finish()`, no further methods may be called.
pub struct LowLevelCursor<'a> {
    pub(crate) base: CursorBase<'a>,
    finished: bool,
}

impl<'a> LowLevelCursor<'a> {
    fn assert_not_finished(&self) {
        assert!(!self.finished, "LowLevelCursor used after finish()");
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
            let raw = self.base.reader.raw();
            let result = ffi::syntaqlite_parser_result(raw);
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
    unsafe fn extract_error(result: &ffi::ParseResult) -> ParseError {
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
        let raw = self.base.reader.raw();
        let rc = unsafe {
            let c_text = self.base.c_source_ptr.add(span.start);
            ffi::syntaqlite_parser_feed_token(
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
        let rc = unsafe { ffi::syntaqlite_parser_finish(self.base.reader.raw()) };
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
        let raw = self.base.reader.raw();
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

    /// Return the semantic completion context at the parser's current state.
    ///
    /// Returns a raw u32: 0 = Unknown, 1 = Expression, 2 = TableRef.
    pub fn completion_context(&self) -> u32 {
        self.assert_not_finished();
        // SAFETY: raw is valid and exclusively borrowed via &self.
        unsafe { ffi::syntaqlite_parser_completion_context(self.base.reader.raw()) }
    }

    /// Return the number of nodes currently in the parser arena.
    ///
    /// Flushes any pending list nodes first, so all node data is consistent.
    pub fn node_count(&self) -> u32 {
        // SAFETY: raw is valid and exclusively borrowed via &self.
        unsafe { ffi::syntaqlite_parser_node_count(self.base.reader.raw()) }
    }

    /// Mark subsequent fed tokens as being inside a macro expansion.
    ///
    /// `call_offset` and `call_length` describe the macro call's byte range
    /// in the original source. Calls may nest (for nested macro expansions).
    pub fn begin_macro(&mut self, call_offset: u32, call_length: u32) {
        self.assert_not_finished();
        // SAFETY: raw is valid and exclusively borrowed via &mut self.
        unsafe {
            ffi::syntaqlite_parser_begin_macro(self.base.reader.raw(), call_offset, call_length);
        }
    }

    /// End the innermost macro expansion region.
    pub fn end_macro(&mut self) {
        self.assert_not_finished();
        // SAFETY: raw is valid and exclusively borrowed via &mut self.
        unsafe {
            ffi::syntaqlite_parser_end_macro(self.base.reader.raw());
        }
    }

    /// Access the underlying `CursorBase` for read-only operations.
    pub fn base(&self) -> &CursorBase<'a> {
        &self.base
    }
}

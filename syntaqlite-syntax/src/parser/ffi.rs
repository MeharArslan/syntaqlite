// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::ffi::{c_char, c_void};

/// Opaque C parser type.
pub(crate) enum CParser {}

/// Return code: no statement / done.
pub(crate) const PARSE_DONE: i32 = 0;
/// Return code: statement parsed cleanly.
pub(crate) const PARSE_OK: i32 = 1;
/// Return code: statement parsed with error recovery.
#[allow(dead_code)]
pub(crate) const PARSE_RECOVERED: i32 = 2;
/// Return code: unrecoverable error.
#[allow(dead_code)]
pub(crate) const PARSE_ERROR: i32 = -1;

/// Mirrors C `SyntaqliteMemMethods`.
#[repr(C)]
pub(crate) struct CMemMethods {
    pub x_malloc: unsafe extern "C" fn(usize) -> *mut c_void,
    pub x_free: unsafe extern "C" fn(*mut c_void),
}

/// The kind of a comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(dead_code)]
pub(crate) enum CCommentKind {
    LineComment = 0,
    BlockComment = 1,
}

/// Mirrors C `SyntaqliteComment`.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(crate) struct CComment {
    pub offset: u32,
    pub length: u32,
    pub kind: CCommentKind,
}

#[allow(dead_code)]
pub(super) const TOKEN_FLAG_AS_ID: u32 = 1;
#[allow(dead_code)]
pub(super) const TOKEN_FLAG_AS_FUNCTION: u32 = 2;
#[allow(dead_code)]
pub(super) const TOKEN_FLAG_AS_TYPE: u32 = 4;

/// Mirrors C `SyntaqliteCompletionContext` (`typedef uint32_t`).
pub(crate) type CCompletionContext = u32;

/// Mirrors C `SyntaqliteParserTokenFlags` (`typedef uint32_t`).
pub(crate) type CParserTokenFlags = u32;

/// Mirrors C `SyntaqliteParserToken`.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(crate) struct CParserToken {
    pub offset: u32,
    pub length: u32,
    pub type_: u32,
    pub flags: CParserTokenFlags,
}

/// A recorded macro invocation region.
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

/// Tag value for error placeholder nodes (tag 0).
#[allow(dead_code)]
pub(super) const SYNTAQLITE_ERROR_NODE_TAG: u32 = 0;

/// Mirrors C `SyntaqliteErrorNode`.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(super) struct CErrorNode {
    pub tag: u32,
    pub offset: u32,
    pub length: u32,
}
use std::mem::size_of;
const _: () = assert!(size_of::<CErrorNode>() == 12);

impl CParser {
    // Lifecycle
    pub(crate) unsafe fn create(
        mem: *const CMemMethods,
        grammar: crate::grammar::ffi::CGrammar,
    ) -> *mut Self {
        // SAFETY: mem may be null (use default allocator); grammar is a
        // valid grammar handle passed by the caller.
        unsafe { syntaqlite_parser_create_with_grammar(mem, grammar) }
    }

    pub(crate) unsafe fn set_trace(&mut self, enable: u32) -> i32 {
        // SAFETY: self is a valid, non-null CParser pointer owned by the caller.
        unsafe { syntaqlite_parser_set_trace(self, enable) }
    }

    pub(crate) unsafe fn set_collect_tokens(&mut self, enable: u32) -> i32 {
        // SAFETY: self is a valid, non-null CParser pointer owned by the caller.
        unsafe { syntaqlite_parser_set_collect_tokens(self, enable) }
    }

    pub(crate) unsafe fn reset(&mut self, source: *const c_char, len: u32) {
        // SAFETY: self is a valid, non-null CParser pointer; source is a
        // null-terminated C string of at least `len` bytes.
        unsafe { syntaqlite_parser_reset(self, source, len) }
    }

    pub(crate) unsafe fn next(&mut self) -> i32 {
        // SAFETY: self is a valid, non-null CParser pointer owned by the caller.
        unsafe { syntaqlite_parser_next(self) }
    }

    pub(crate) unsafe fn destroy(this: *mut Self) {
        // SAFETY: this is a valid CParser pointer previously created by
        // `syntaqlite_parser_create_with_grammar` and not yet destroyed.
        unsafe { syntaqlite_parser_destroy(this) }
    }

    // Result accessors (valid after `next()` returns non-DONE)
    pub(crate) unsafe fn result_root(&self) -> u32 {
        // SAFETY: self is a valid, non-null CParser pointer; result
        // accessors are valid after `next()` returns a non-DONE code.
        unsafe { syntaqlite_result_root(std::ptr::from_ref::<Self>(self).cast_mut()) }
    }

    pub(crate) unsafe fn result_error_kind(&self) -> u32 {
        // SAFETY: self is a valid, non-null CParser pointer; result
        // accessors are valid after `next()` returns a non-DONE code.
        unsafe { syntaqlite_result_error_kind(std::ptr::from_ref::<Self>(self).cast_mut()) }
    }

    pub(crate) unsafe fn result_error_msg(&self) -> *const c_char {
        // SAFETY: self is a valid, non-null CParser pointer; result
        // accessors are valid after `next()` returns a non-DONE code.
        unsafe { syntaqlite_result_error_msg(std::ptr::from_ref::<Self>(self).cast_mut()) }
    }

    pub(crate) unsafe fn result_error_offset(&self) -> u32 {
        // SAFETY: self is a valid, non-null CParser pointer; result
        // accessors are valid after `next()` returns a non-DONE code.
        unsafe { syntaqlite_result_error_offset(std::ptr::from_ref::<Self>(self).cast_mut()) }
    }

    pub(crate) unsafe fn result_error_length(&self) -> u32 {
        // SAFETY: self is a valid, non-null CParser pointer; result
        // accessors are valid after `next()` returns a non-DONE code.
        unsafe { syntaqlite_result_error_length(std::ptr::from_ref::<Self>(self).cast_mut()) }
    }

    pub(crate) unsafe fn result_comments(&self) -> &[CComment] {
        let mut count: u32 = 0;
        // SAFETY: self is a valid, non-null CParser pointer; result
        // accessors are valid after `next()` returns a non-DONE code.
        let ptr = unsafe {
            syntaqlite_result_comments(std::ptr::from_ref::<Self>(self).cast_mut(), &raw mut count)
        };
        if count == 0 || ptr.is_null() {
            return &[];
        }
        // SAFETY: ptr is a valid pointer to `count` CComment values owned
        // by the parser arena; the slice is valid for the parser's lifetime.
        unsafe { std::slice::from_raw_parts(ptr, count as usize) }
    }

    pub(crate) unsafe fn result_tokens(&self) -> &[CParserToken] {
        let mut count: u32 = 0;
        // SAFETY: self is a valid, non-null CParser pointer; result
        // accessors are valid after `next()` returns a non-DONE code.
        let ptr = unsafe {
            syntaqlite_result_tokens(std::ptr::from_ref::<Self>(self).cast_mut(), &raw mut count)
        };
        if count == 0 || ptr.is_null() {
            return &[];
        }
        // SAFETY: ptr is a valid pointer to `count` CParserToken values owned
        // by the parser arena; the slice is valid for the parser's lifetime.
        unsafe { std::slice::from_raw_parts(ptr, count as usize) }
    }

    pub(crate) unsafe fn result_macros(&self) -> &[CMacroRegion] {
        let mut count: u32 = 0;
        // SAFETY: self is a valid, non-null CParser pointer; result
        // accessors are valid after `next()` returns a non-DONE code.
        let ptr = unsafe {
            syntaqlite_result_macros(std::ptr::from_ref::<Self>(self).cast_mut(), &raw mut count)
        };
        if count == 0 || ptr.is_null() {
            return &[];
        }
        // SAFETY: ptr is a valid pointer to `count` CMacroRegion values owned
        // by the parser arena; the slice is valid for the parser's lifetime.
        unsafe { std::slice::from_raw_parts(ptr, count as usize) }
    }

    // Arena accessors
    pub(crate) unsafe fn node(&self, node_id: u32) -> *const u32 {
        // SAFETY: self is a valid, non-null CParser pointer; node_id is a
        // raw node ID from the arena (null is handled by the C side).
        unsafe { syntaqlite_parser_node(std::ptr::from_ref::<Self>(self).cast_mut(), node_id) }
    }

    pub(crate) unsafe fn node_count(&self) -> u32 {
        // SAFETY: self is a valid, non-null CParser pointer owned by the caller.
        unsafe { syntaqlite_parser_node_count(std::ptr::from_ref::<Self>(self).cast_mut()) }
    }

    // AST dump
    pub(crate) unsafe fn dump_node(&self, node_id: u32, indent: u32) -> *mut c_char {
        // SAFETY: self is a valid, non-null CParser pointer; node_id is a
        // raw node ID from the arena. Returns a malloc'd string or null.
        unsafe { syntaqlite_dump_node(std::ptr::from_ref::<Self>(self).cast_mut(), node_id, indent) }
    }

    // Incremental (token-feeding) API
    pub(crate) unsafe fn feed_token(&mut self, token_type: u32, text: *const c_char, len: u32) -> i32 {
        // SAFETY: self is a valid, non-null CParser pointer; text is a
        // valid pointer to at least `len` bytes of token text.
        unsafe { syntaqlite_parser_feed_token(self, token_type, text, len) }
    }

    pub(crate) unsafe fn finish(&mut self) -> i32 {
        // SAFETY: self is a valid, non-null CParser pointer owned by the caller.
        unsafe { syntaqlite_parser_finish(self) }
    }

    pub(crate) unsafe fn expected_tokens(&self, out_tokens: *mut u32, out_cap: u32) -> u32 {
        // SAFETY: self is a valid, non-null CParser pointer; out_tokens
        // is a valid pointer to at least `out_cap` u32 values.
        unsafe {
            syntaqlite_parser_expected_tokens(
                std::ptr::from_ref::<Self>(self).cast_mut(),
                out_tokens,
                out_cap,
            )
        }
    }

    pub(crate) unsafe fn completion_context(&self) -> super::CompletionContext {
        // SAFETY: self is a valid, non-null CParser pointer owned by the caller.
        unsafe {
            super::CompletionContext::from_raw(syntaqlite_parser_completion_context(
                std::ptr::from_ref::<Self>(self).cast_mut(),
            ))
        }
    }

    pub(crate) unsafe fn begin_macro(&mut self, call_offset: u32, call_length: u32) {
        // SAFETY: self is a valid, non-null CParser pointer owned by the caller.
        unsafe { syntaqlite_parser_begin_macro(self, call_offset, call_length) }
    }

    pub(crate) unsafe fn end_macro(&mut self) {
        // SAFETY: self is a valid, non-null CParser pointer owned by the caller.
        unsafe { syntaqlite_parser_end_macro(self) }
    }
}

unsafe extern "C" {
    // Parser lifecycle
    fn syntaqlite_parser_create_with_grammar(
        mem: *const CMemMethods,
        grammar: crate::grammar::ffi::CGrammar,
    ) -> *mut CParser;
    fn syntaqlite_parser_reset(p: *mut CParser, source: *const c_char, len: u32);
    fn syntaqlite_parser_next(p: *mut CParser) -> i32;
    fn syntaqlite_parser_destroy(p: *mut CParser);

    // Result accessors
    fn syntaqlite_result_root(p: *mut CParser) -> u32;
    fn syntaqlite_result_error_kind(p: *mut CParser) -> u32;
    fn syntaqlite_result_error_msg(p: *mut CParser) -> *const c_char;
    fn syntaqlite_result_error_offset(p: *mut CParser) -> u32;
    fn syntaqlite_result_error_length(p: *mut CParser) -> u32;
    fn syntaqlite_result_comments(p: *mut CParser, count: *mut u32) -> *const CComment;
    fn syntaqlite_result_tokens(p: *mut CParser, count: *mut u32) -> *const CParserToken;
    fn syntaqlite_result_macros(p: *mut CParser, count: *mut u32) -> *const CMacroRegion;

    // Arena accessors
    fn syntaqlite_parser_node(p: *mut CParser, node_id: u32) -> *const u32;
    fn syntaqlite_parser_node_count(p: *mut CParser) -> u32;

    // Configuration
    fn syntaqlite_parser_set_trace(p: *mut CParser, enable: u32) -> i32;
    fn syntaqlite_parser_set_collect_tokens(p: *mut CParser, enable: u32) -> i32;

    // AST dump
    fn syntaqlite_dump_node(p: *mut CParser, node_id: u32, indent: u32) -> *mut c_char;

    // Incremental (token-feeding) API (from incremental.h)
    fn syntaqlite_parser_feed_token(
        p: *mut CParser,
        token_type: u32,
        text: *const c_char,
        len: u32,
    ) -> i32;
    fn syntaqlite_parser_finish(p: *mut CParser) -> i32;
    fn syntaqlite_parser_expected_tokens(p: *mut CParser, out_tokens: *mut u32, out_cap: u32)
    -> u32;
    fn syntaqlite_parser_completion_context(p: *mut CParser) -> CCompletionContext;
    fn syntaqlite_parser_begin_macro(p: *mut CParser, call_offset: u32, call_length: u32);
    fn syntaqlite_parser_end_macro(p: *mut CParser);
}

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::ffi::{c_char, c_int, c_void};

use crate::dialect::ffi::Dialect;

// Opaque C types
pub(crate) enum Parser {}

/// Mirrors C `SyntaqliteParseResult` from `include/syntaqlite/parser.h`.
#[repr(C)]
pub(crate) struct ParseResult {
    pub root: u32,
    pub error: i32,
    pub error_msg: *const c_char,
    pub error_offset: u32,
    pub error_length: u32,
}

/// Mirrors C `SyntaqliteMemMethods` from `include/syntaqlite/config.h`.
#[repr(C)]
pub(crate) struct MemMethods {
    pub x_malloc: unsafe extern "C" fn(usize) -> *mut c_void,
    pub x_free: unsafe extern "C" fn(*mut c_void),
}

/// The kind of a comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CommentKind {
    /// A line comment starting with `--`.
    LineComment = 0,
    /// A block comment delimited by `/* ... */`.
    BlockComment = 1,
}

/// A comment captured during parsing. Comments are sorted by source offset.
///
/// Mirrors C `SyntaqliteComment` from `include/syntaqlite/parser.h`.
/// Layout: (offset: u32, length: u32, kind: u8) — returned directly from
/// the C buffer without copying.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Comment {
    pub offset: u32,
    pub length: u32,
    pub kind: CommentKind,
}

/// Token flags bitfield.
pub const TOKEN_FLAG_AS_ID: u32 = 1;
pub const TOKEN_FLAG_AS_FUNCTION: u32 = 2;
pub const TOKEN_FLAG_AS_TYPE: u32 = 4;

/// A non-whitespace, non-comment token position captured during parsing.
///
/// Mirrors C `SyntaqliteTokenPos` from `include/syntaqlite/parser.h`.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TokenPos {
    pub offset: u32,
    pub length: u32,
    /// Original token type from tokenizer (pre-fallback).
    pub type_: u32,
    /// Bitfield: TOKEN_FLAG_AS_ID / AS_FUNCTION / AS_TYPE.
    pub flags: u32,
}

/// A recorded macro invocation region. Populated via the low-level API
/// (`begin_macro` / `end_macro`). The formatter can use these to reconstruct
/// macro calls from the expanded AST.
///
/// Mirrors C `SyntaqliteMacroRegion` from `include/syntaqlite/parser.h`.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MacroRegion {
    /// Byte offset of the macro call in the original source.
    pub call_offset: u32,
    /// Byte length of the entire macro call.
    pub call_length: u32,
}

// Opaque C tokenizer type
pub(crate) enum Tokenizer {}

/// A single token produced by the C tokenizer.
///
/// Mirrors C `SyntaqliteToken` from `include/syntaqlite/tokenizer.h`.
#[repr(C)]
pub(crate) struct Token {
    pub text: *const c_char,
    pub length: u32,
    pub type_: u32,
}

// The C API uses `SyntaqliteNode*` as an opaque return. We only read via
// the tag field (first u32) and then cast to the right struct, so we just
// receive `*const u32`.

unsafe extern "C" {
    // Parser lifecycle
    pub fn syntaqlite_create_parser_with_dialect(
        mem: *const MemMethods,
        dialect: *const Dialect,
    ) -> *mut Parser;
    pub fn syntaqlite_parser_reset(p: *mut Parser, source: *const c_char, len: u32);
    pub fn syntaqlite_parser_next(p: *mut Parser) -> ParseResult;
    pub fn syntaqlite_parser_destroy(p: *mut Parser);

    // Parser accessors
    pub fn syntaqlite_parser_node(p: *mut Parser, node_id: u32) -> *const u32;

    // Parser configuration
    pub fn syntaqlite_parser_set_trace(p: *mut Parser, enable: c_int) -> c_int;
    pub fn syntaqlite_parser_set_collect_tokens(p: *mut Parser, enable: c_int) -> c_int;
    pub fn syntaqlite_parser_set_dialect_config(
        p: *mut Parser,
        config: *const crate::dialect::ffi::DialectConfig,
    ) -> c_int;

    // Comments
    pub fn syntaqlite_parser_comments(p: *mut Parser, count: *mut u32) -> *const Comment;

    // Token positions
    pub fn syntaqlite_parser_tokens(p: *mut Parser, count: *mut u32) -> *const TokenPos;

    // Low-level token-feeding API
    pub fn syntaqlite_parser_feed_token(
        p: *mut Parser,
        token_type: c_int,
        text: *const c_char,
        len: c_int,
    ) -> c_int;
    pub fn syntaqlite_parser_result(p: *mut Parser) -> ParseResult;
    pub fn syntaqlite_parser_expected_tokens(
        p: *mut Parser,
        out_tokens: *mut c_int,
        out_cap: c_int,
    ) -> c_int;
    pub fn syntaqlite_parser_finish(p: *mut Parser) -> c_int;

    // Macro region tracking
    pub fn syntaqlite_parser_begin_macro(p: *mut Parser, call_offset: u32, call_length: u32);
    pub fn syntaqlite_parser_end_macro(p: *mut Parser);
    pub fn syntaqlite_parser_macro_regions(p: *mut Parser, count: *mut u32) -> *const MacroRegion;

    // AST dump
    pub fn syntaqlite_dump_node(p: *mut Parser, node_id: u32, indent: u32) -> *mut c_char;

    // Tokenizer
    pub fn syntaqlite_tokenizer_create(
        mem: *const MemMethods,
        dialect: *const crate::dialect::ffi::Dialect,
    ) -> *mut Tokenizer;
    pub fn syntaqlite_tokenizer_reset(tok: *mut Tokenizer, source: *const c_char, len: u32);
    pub fn syntaqlite_tokenizer_next(tok: *mut Tokenizer, out: *mut Token) -> c_int;
    pub fn syntaqlite_tokenizer_destroy(tok: *mut Tokenizer);
    pub fn syntaqlite_tokenizer_set_dialect_config(
        tok: *mut Tokenizer,
        config: *const crate::dialect::ffi::DialectConfig,
    ) -> c_int;

}

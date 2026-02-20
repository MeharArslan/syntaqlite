// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::ffi::{CStr, c_int};

use super::ffi;
use super::ffi::{Comment, MacroRegion};
use super::nodes::{NodeId, NodeList};
use crate::dialect::Dialect;

/// A parse error with a human-readable message.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

/// Configuration for a parser. Must be set before first use (before `parse()`).
#[derive(Debug, Clone, Default)]
pub struct ParserConfig {
    pub trace: bool,
    pub collect_tokens: bool,
}

/// Owns a parser instance. Reusable across inputs via `parse()`.
pub struct Parser {
    pub(crate) raw: *mut ffi::Parser,
    /// Null-terminated copy of the source text. The C tokenizer (SQLite's
    /// `SynqSqliteGetToken`) reads until it hits a null byte, so we must
    /// ensure the source is null-terminated. Rust `&str` does not guarantee
    /// this. The buffer is reused across `parse()` calls to avoid repeated
    /// allocations.
    pub(crate) source_buf: Vec<u8>,
    config: ParserConfig,
}

// SAFETY: The C parser is self-contained (no thread-local or shared mutable
// state). Moving it between threads is safe; concurrent access is prevented
// by &mut borrowing in parse().
unsafe impl Send for Parser {}

impl Parser {
    pub fn new(dialect: &Dialect) -> Self {
        // SAFETY: syntaqlite_create_parser_with_dialect(NULL, dialect) allocates
        // a new parser with default malloc/free. It always succeeds.
        let raw =
            unsafe { ffi::syntaqlite_create_parser_with_dialect(std::ptr::null(), dialect.raw) };
        assert!(!raw.is_null(), "parser allocation failed");
        Parser {
            raw,
            source_buf: Vec::new(),
            config: ParserConfig::default(),
        }
    }

    /// Create a parser with the given configuration applied at construction.
    pub fn with_config(dialect: &Dialect, config: &ParserConfig) -> Self {
        let mut parser = Self::new(dialect);
        // SAFETY: Parser is freshly created (not sealed), so these calls
        // always return 0.
        unsafe {
            ffi::syntaqlite_parser_set_trace(parser.raw, config.trace as c_int);
            ffi::syntaqlite_parser_set_collect_tokens(parser.raw, config.collect_tokens as c_int);
        }
        parser.config = config.clone();
        parser
    }

    /// Access the current configuration.
    pub fn config(&self) -> &ParserConfig {
        &self.config
    }

    /// Bind source text and return a `StatementCursor` for iterating statements.
    ///
    /// Copies the source into an internal buffer to add a null terminator
    /// (required by the C tokenizer). For zero-copy parsing, use
    /// [`parse_cstr`](Self::parse_cstr).
    pub fn parse<'a>(&'a mut self, source: &'a str) -> StatementCursor<'a> {
        let base = CursorBase::new(self.raw, &mut self.source_buf, source);
        StatementCursor(base)
    }

    /// Zero-copy variant: bind a null-terminated source and return a
    /// `StatementCursor`.
    ///
    /// The `&CStr` already guarantees a trailing `\0`, so no copy is needed.
    /// The source must be valid UTF-8 (panics otherwise).
    pub fn parse_cstr<'a>(&'a mut self, source: &'a CStr) -> StatementCursor<'a> {
        let base = CursorBase::new_cstr(self.raw, source);
        StatementCursor(base)
    }
}

impl Drop for Parser {
    fn drop(&mut self) {
        // SAFETY: self.raw was allocated by syntaqlite_parser_create and has
        // not been freed (Drop runs exactly once). The C function is no-op
        // on NULL.
        unsafe { ffi::syntaqlite_parser_destroy(self.raw) }
    }
}

// ── NodeReader ──────────────────────────────────────────────────────────

/// A lightweight, `Copy` handle for reading nodes from the parser arena.
///
/// This is the read-only half of `CursorBase`. Dialect crates embed it in
/// view structs so that accessor methods can resolve `NodeId` children
/// without requiring a back-reference to the full cursor.
///
/// # Safety invariant
/// The raw pointer must remain valid for `'a`. This is guaranteed when
/// `NodeReader` is obtained from a `CursorBase` (which borrows the parser
/// exclusively for `'a`).
#[derive(Clone, Copy)]
pub struct NodeReader<'a> {
    raw: *mut ffi::Parser,
    source: &'a str,
}

impl<'a> NodeReader<'a> {
    /// Get a raw pointer to a node in the arena. Returns `(pointer, tag)`.
    pub fn node_ptr(&self, id: NodeId) -> Option<(*const u8, u32)> {
        if id.is_null() {
            return None;
        }
        unsafe {
            let ptr = ffi::syntaqlite_parser_node(self.raw, id.0);
            if ptr.is_null() {
                return None;
            }
            let tag = *(ptr as *const u32);
            Some((ptr as *const u8, tag))
        }
    }

    /// The source text bound to this reader.
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// Access the raw C parser pointer (crate-internal).
    pub(crate) fn raw(&self) -> *mut ffi::Parser {
        self.raw
    }

    /// Dump an AST node tree as indented text. Uses C-side metadata (field
    /// names, display strings) so no Rust-side string tables are needed.
    pub fn dump_node(&self, id: NodeId, out: &mut String, indent: usize) {
        unsafe extern "C" {
            fn free(ptr: *mut std::ffi::c_void);
        }
        // SAFETY: raw is valid; dump_node returns a malloc'd NUL-terminated
        // string (or null). We free the C string after copying.
        unsafe {
            let ptr = ffi::syntaqlite_dump_node(self.raw, id.0, indent as u32);
            if !ptr.is_null() {
                let cstr = CStr::from_ptr(ptr);
                out.push_str(&cstr.to_string_lossy());
                free(ptr as *mut std::ffi::c_void);
            }
        }
    }
}

// ── CursorBase ──────────────────────────────────────────────────────────

/// Shared read-only cursor state. Both `StatementCursor` and `LowLevelCursor`
/// wrap this.
pub struct CursorBase<'a> {
    pub(crate) reader: NodeReader<'a>,
    /// The pointer that the C parser uses as its source base. This may differ
    /// from `source.as_ptr()` when `parse()` copies into an internal buffer.
    /// `feed_token` translates user text pointers through this so that the C
    /// code's `tok.z - ctx->source` offset arithmetic is correct regardless
    /// of whether the copying or zero-copy path was used.
    pub(crate) c_source_ptr: *const u8,
}

impl<'a> CursorBase<'a> {
    /// Construct a CursorBase from a raw parser pointer and source text.
    /// Copies the source into `source_buf` to null-terminate it, then resets
    /// the C parser.
    pub(crate) fn new(raw: *mut ffi::Parser, source_buf: &'a mut Vec<u8>, source: &'a str) -> Self {
        source_buf.clear();
        source_buf.reserve(source.len() + 1);
        source_buf.extend_from_slice(source.as_bytes());
        source_buf.push(0);

        let c_source_ptr = source_buf.as_ptr();
        unsafe {
            ffi::syntaqlite_parser_reset(raw, c_source_ptr as *const _, source.len() as u32);
        }
        CursorBase {
            reader: NodeReader { raw, source },
            c_source_ptr,
        }
    }

    /// Construct a CursorBase from a raw parser pointer and a CStr (zero-copy).
    pub(crate) fn new_cstr(raw: *mut ffi::Parser, source: &'a CStr) -> Self {
        let bytes = source.to_bytes();
        let source_str = std::str::from_utf8(bytes).expect("source must be valid UTF-8");

        unsafe {
            ffi::syntaqlite_parser_reset(raw, source.as_ptr(), bytes.len() as u32);
        }
        CursorBase {
            reader: NodeReader {
                raw,
                source: source_str,
            },
            c_source_ptr: source.as_ptr() as *const u8,
        }
    }

    /// Get a reference to the embedded `NodeReader`.
    ///
    /// The returned reference borrows `self`, so nodes resolved through it
    /// cannot outlive this cursor.
    pub fn reader(&self) -> &NodeReader<'a> {
        &self.reader
    }

    /// Get a raw pointer to a node in the arena. Returns (pointer, tag).
    ///
    /// This is the dialect-agnostic primitive. Dialect crates wrap this to
    /// return a typed `Node` enum.
    pub fn node_ptr(&self, id: NodeId) -> Option<(*const u8, u32)> {
        self.reader.node_ptr(id)
    }

    /// The source text bound to this cursor.
    pub fn source(&self) -> &'a str {
        self.reader.source()
    }

    /// If `id` refers to a list node, return its child node IDs.
    pub fn list_children(&self, id: NodeId, dialect: &Dialect) -> Option<&[NodeId]> {
        let (ptr, tag) = self.node_ptr(id)?;
        if !dialect.is_list(tag) {
            return None;
        }
        // SAFETY: ptr is a valid arena pointer and the tag confirms it is a
        // list node, so it has NodeList layout (tag, count, children[count]).
        Some(unsafe { &*(ptr as *const NodeList) }.children())
    }

    /// Return all comments captured during parsing.
    /// Requires `collect_tokens: true` in `ParserConfig`.
    ///
    /// Returns a slice into the parser's internal buffer — valid until
    /// the parser is reset or destroyed (which requires `&mut`).
    pub fn comments(&self) -> &[Comment] {
        // SAFETY: raw is valid; the returned pointer and count are valid for
        // the lifetime of &self (until the next reset/destroy, which need &mut).
        unsafe {
            let mut count: u32 = 0;
            let ptr = ffi::syntaqlite_parser_comments(self.reader.raw(), &mut count);
            if count == 0 || ptr.is_null() {
                return &[];
            }
            std::slice::from_raw_parts(ptr, count as usize)
        }
    }

    /// Dump an AST node tree as indented text. Uses C-side metadata (field
    /// names, display strings) so no Rust-side string tables are needed.
    pub fn dump_node(&self, id: NodeId, out: &mut String, indent: usize) {
        self.reader.dump_node(id, out, indent)
    }

    /// Return all macro regions recorded via `begin_macro`/`end_macro`.
    pub fn macro_regions(&self) -> &[MacroRegion] {
        // SAFETY: same as comments() — pointer valid for lifetime of &self.
        unsafe {
            let mut count: u32 = 0;
            let ptr = ffi::syntaqlite_parser_macro_regions(self.reader.raw(), &mut count);
            if count == 0 || ptr.is_null() {
                return &[];
            }
            std::slice::from_raw_parts(ptr, count as usize)
        }
    }
}

// ── StatementCursor (high-level) ────────────────────────────────────────

/// A streaming cursor over parsed SQL statements. Iterate with
/// `next_statement()` or the `Iterator` impl.
pub struct StatementCursor<'a>(pub(crate) CursorBase<'a>);

impl<'a> StatementCursor<'a> {
    /// Parse the next SQL statement. Returns `None` when all statements have
    /// been consumed (or input was empty).
    pub fn next_statement(&mut self) -> Option<Result<NodeId, ParseError>> {
        // SAFETY: raw is valid and exclusively borrowed via &mut self.
        // When error is set, error_msg is a NUL-terminated string in the
        // parser's buffer (valid for parser lifetime).
        let result = unsafe { ffi::syntaqlite_parser_next(self.0.reader.raw()) };

        let id = NodeId(result.root);
        if !id.is_null() {
            return Some(Ok(id));
        }

        if result.error != 0 {
            let msg = unsafe { CStr::from_ptr(result.error_msg) }
                .to_string_lossy()
                .into_owned();
            return Some(Err(ParseError { message: msg }));
        }

        None
    }

    /// Access the underlying `CursorBase` for read-only operations.
    pub fn base(&self) -> &CursorBase<'a> {
        &self.0
    }

    // Delegate read-only methods for convenience

    /// Get a reference to the embedded `NodeReader`.
    pub fn reader(&self) -> &NodeReader<'a> {
        self.0.reader()
    }

    /// The source text bound to this cursor.
    pub fn source(&self) -> &'a str {
        self.0.source()
    }

    /// Dump an AST node tree as indented text.
    pub fn dump_node(&self, id: NodeId, out: &mut String, indent: usize) {
        self.0.dump_node(id, out, indent)
    }
}

impl Iterator for StatementCursor<'_> {
    type Item = Result<NodeId, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_statement()
    }
}

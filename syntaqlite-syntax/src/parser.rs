// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::RefCell;
use std::ffi::{CStr, c_int};
use std::ptr::NonNull;
use std::rc::Rc;

use crate::ast::{
    ArenaNode, FIELD_BOOL, FIELD_ENUM, FIELD_FLAGS, FIELD_NODE_ID, FIELD_SPAN, FieldVal, Fields,
    GrammarNodeType, NodeList, RawNode, RawNodeId, SourceSpan,
};
use crate::grammar::RawGrammar;

// ── Public API ───────────────────────────────────────────────────────────────

// ── ParserConfig ─────────────────────────────────────────────────────────────

/// Configuration for parser construction.
#[derive(Debug, Default, Clone, Copy)]
pub struct ParserConfig {
    /// Enable parser trace output (Lemon debug trace). Default: `false`.
    pub trace: bool,
    /// Collect non-whitespace token positions during parsing. Default: `false`.
    pub collect_tokens: bool,
}

// ── Parser ───────────────────────────────────────────────────────────────────

/// Owns a parser instance. Reusable across inputs via `parse()` and
/// `incremental_parse()`.
///
/// Uses an interior-mutability checkout pattern: `parse()` and
/// `incremental_parse()` check out the C parser state at runtime, and the
/// returned cursor returns it on drop. This allows both methods to take
/// `&self` rather than `&mut self`.
pub struct Parser {
    inner: Rc<RefCell<Option<ParserInner>>>,
    grammar: RawGrammar,
}

impl Parser {
    /// Create a parser bound to the given grammar with default configuration.
    pub fn new(grammar: RawGrammar) -> Self {
        Self::with_config(grammar, &ParserConfig::default())
    }

    /// Create a parser bound to the given grammar with custom configuration.
    pub fn with_config(grammar: RawGrammar, config: &ParserConfig) -> Self {
        // SAFETY: syntaqlite_create_parser_with_grammar(NULL, grammar.inner) allocates
        // a new parser with default malloc/free. The C side copies the grammar.
        let raw = NonNull::new(unsafe {
            ffi::syntaqlite_create_parser_with_grammar(std::ptr::null(), grammar.inner)
        })
        .expect("parser allocation failed");

        // SAFETY: raw is freshly created (not sealed), so these calls
        // always return 0.
        unsafe {
            ffi::syntaqlite_parser_set_trace(raw.as_ptr(), config.trace as c_int);
            ffi::syntaqlite_parser_set_collect_tokens(raw.as_ptr(), config.collect_tokens as c_int);
        }

        let inner = ParserInner {
            raw,
            source_buf: Vec::new(),
        };

        Parser {
            inner: Rc::new(RefCell::new(Some(inner))),
            grammar,
        }
    }

    /// Bind source text and return a [`StatementCursor`] for iterating
    /// statements.
    ///
    /// Copies the source into an internal buffer to add a null terminator
    /// (required by the C tokenizer). The cursor owns the copy, so the
    /// original `source` does not need to outlive the cursor.
    ///
    /// # Panics
    ///
    /// Panics if a cursor from a previous `parse()` or `incremental_parse()`
    /// call is still alive.
    pub fn parse(&self, source: &str) -> StatementCursor {
        let mut inner = self
            .inner
            .borrow_mut()
            .take()
            .expect("Parser::parse called while a cursor is still active");
        // SAFETY: inner.raw is valid (owned via ParserInner); source is
        // copied into source_buf which will be owned by the cursor.
        unsafe { reset_parser(inner.raw.as_ptr(), &mut inner.source_buf, source) };
        StatementCursor {
            grammar: self.grammar,
            inner: Some(inner),
            slot: Rc::clone(&self.inner),
            last_saw_subquery: false,
            last_saw_update_delete_limit: false,
        }
    }

    /// Bind source text and return an [`IncrementalCursor`](crate::incremental::IncrementalCursor)
    /// for token-by-token feeding.
    ///
    /// Copies the source into an internal buffer to add a null terminator
    /// (required by the C tokenizer). The cursor owns the copy, so the
    /// original `source` does not need to outlive the cursor.
    ///
    /// # Panics
    ///
    /// Panics if a cursor from a previous `parse()` or `incremental_parse()`
    /// call is still alive.
    pub fn incremental_parse(&self, source: &str) -> crate::incremental::IncrementalCursor {
        let mut inner = self
            .inner
            .borrow_mut()
            .take()
            .expect("Parser::incremental_parse called while a cursor is still active");
        // SAFETY: inner.raw is valid (owned via ParserInner); source is
        // copied into source_buf.
        unsafe { reset_parser(inner.raw.as_ptr(), &mut inner.source_buf, source) };
        let c_source_ptr =
            NonNull::new(inner.source_buf.as_mut_ptr()).expect("source_buf is non-empty");
        crate::incremental::IncrementalCursor::new(
            c_source_ptr,
            self.grammar,
            inner,
            Rc::clone(&self.inner),
        )
    }
}

// ── StatementCursor ──────────────────────────────────────────────────────────

/// A streaming cursor over parsed SQL statements.
///
/// On a parse error the cursor returns `Some(Err(_))` for the failing
/// statement, then continues parsing subsequent statements (Lemon's built-in
/// error recovery synchronises on `;`). Call `next_statement()` again to
/// retrieve the next valid statement.
///
/// On drop, the checked-out parser state is returned to the parent [`Parser`].
pub struct StatementCursor {
    grammar: RawGrammar,
    /// Checked-out parser state. Returned to `slot` on drop.
    inner: Option<ParserInner>,
    /// Slot to return `inner` to when this cursor is dropped.
    slot: Rc<RefCell<Option<ParserInner>>>,
    /// Value of `saw_subquery` from the last successful `next_statement()` call.
    last_saw_subquery: bool,
    /// Value of `saw_update_delete_limit` from the last successful `next_statement()` call.
    last_saw_update_delete_limit: bool,
}

impl Drop for StatementCursor {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            *self.slot.borrow_mut() = Some(inner);
        }
    }
}

impl StatementCursor {
    /// Parse the next SQL statement.
    ///
    /// Returns:
    /// - `Some(Ok(node))` — successfully parsed statement root.
    /// - `Some(Err(e))` — syntax error; call again to continue with subsequent
    ///   statements (Lemon recovers on `;`).
    /// - `None` — all input has been consumed.
    pub(crate) fn next_statement(&mut self) -> Option<Result<RawNode<'_>, ParseError>> {
        // SAFETY: raw is valid and exclusively borrowed via &mut self.
        // When error is set, error_msg is a NUL-terminated string in the
        // parser's buffer (valid for parser lifetime).
        let result = unsafe { ffi::syntaqlite_parser_next(self.raw_ptr()) };

        let id = RawNodeId(result.root);
        let has_root = !id.is_null();
        let has_error = result.error != 0;

        if has_error {
            // SAFETY: error_msg is a NUL-terminated string in the parser's
            // buffer (valid for parser lifetime), guaranteed when error != 0.
            let msg = unsafe { CStr::from_ptr(result.error_msg) }
                .to_string_lossy()
                .into_owned();
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
            let root = if has_root {
                self.last_saw_subquery = result.saw_subquery != 0;
                self.last_saw_update_delete_limit = result.saw_update_delete_limit != 0;
                Some(id)
            } else {
                None
            };
            return Some(Err(ParseError {
                message: msg,
                offset,
                length,
                root,
            }));
        }

        if has_root {
            self.last_saw_subquery = result.saw_subquery != 0;
            self.last_saw_update_delete_limit = result.saw_update_delete_limit != 0;
            return Some(Ok(RawNode::new(id, self.reader(), self.grammar)));
        }

        None
    }

    /// Build a [`ParseResult`] for the parser arena, borrowing source text
    /// from the internal buffer.
    ///
    /// Lightweight (no allocation) — packages the raw parser pointer with a
    /// `&str` view of the owned source buffer.
    pub(crate) fn reader(&self) -> ParseResult<'_> {
        let inner = self.inner.as_ref().unwrap();
        let source_len = inner.source_buf.len().saturating_sub(1);
        // SAFETY: source_buf was populated from valid UTF-8 (&str) in
        // reset_parser. The first source_len bytes are the original source.
        let source = unsafe { std::str::from_utf8_unchecked(&inner.source_buf[..source_len]) };
        // SAFETY: inner.raw is valid (owned via ParserInner, not yet destroyed).
        unsafe { ParseResult::new(inner.raw.as_ptr(), source) }
    }

    /// The source text bound to this cursor.
    pub fn source(&self) -> &str {
        let inner = self.inner.as_ref().unwrap();
        let source_len = inner.source_buf.len().saturating_sub(1);
        // SAFETY: source_buf was populated from valid UTF-8 (&str) in
        // reset_parser.
        unsafe { std::str::from_utf8_unchecked(&inner.source_buf[..source_len]) }
    }

    /// The grammar for this cursor.
    pub fn grammar(&self) -> RawGrammar {
        self.grammar
    }

    /// Return all non-whitespace, non-comment token positions captured
    /// during parsing. Requires `collect_tokens: true` in [`ParserConfig`].
    pub(crate) fn tokens(&self) -> &[ffi::CTokenPos] {
        // SAFETY: raw is valid (owned via ParserInner, valid for &self).
        unsafe { ffi_slice(self.raw_ptr(), ffi::syntaqlite_parser_tokens) }
    }

    /// Return all comments captured during parsing.
    /// Requires `collect_tokens: true` in [`ParserConfig`].
    pub(crate) fn comments(&self) -> &[ffi::CComment] {
        // SAFETY: raw is valid (owned via ParserInner, valid for &self).
        unsafe { ffi_slice(self.raw_ptr(), ffi::syntaqlite_parser_comments) }
    }

    /// Wrap a `RawNodeId` (e.g. from a `ParseError::root`) into a [`RawNode`]
    /// using this cursor's reader and grammar.
    pub(crate) fn node_ref(&self, id: RawNodeId) -> RawNode<'_> {
        RawNode::new(id, self.reader(), self.grammar)
    }

    /// The raw C parser pointer from the checked-out inner state.
    fn raw_ptr(&self) -> *mut ffi::CParser {
        self.inner.as_ref().unwrap().raw.as_ptr()
    }
}

// ── ParseError ───────────────────────────────────────────────────────────────

/// A parse error with a human-readable message and optional source location.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    /// Byte offset of the error token in the source text.
    /// `None` if the error location is unknown.
    pub offset: Option<usize>,
    /// Byte length of the error token.
    /// `None` if the error length is unknown.
    pub length: Option<usize>,
    /// Root node of a partially recovered parse tree, if error recovery
    /// succeeded. The tree may contain [`ffi::CErrorNode`] placeholders (tag 0)
    /// in positions where the parser recovered. `None` when the error was
    /// unrecoverable and no tree was produced.
    pub root: Option<RawNodeId>,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

// ── Crate-internal ───────────────────────────────────────────────────────────

// ── ParseResult ──────────────────────────────────────────────────────────────

/// A lightweight, `Copy` handle for reading the parser arena: nodes, spans,
/// and field data.
///
/// This is the read-only half of a cursor state. Dialect crates embed it in
/// view structs so that accessor methods can resolve child node IDs without
/// requiring a back-reference to the full cursor.
///
/// # Safety invariant
/// The raw pointer must remain valid for `'a`. This is guaranteed when
/// `ParseResult` is obtained from a cursor (which borrows the parser
/// exclusively for `'a`).
#[derive(Clone, Copy)]
pub(crate) struct ParseResult<'a> {
    pub(crate) raw: NonNull<ffi::CParser>,
    pub(crate) source: &'a str,
}

impl<'a> ParseResult<'a> {
    /// Construct a `ParseResult` from a raw parser pointer and source text.
    ///
    /// # Safety
    /// `raw` must be a valid, non-null parser pointer that remains valid
    /// for the lifetime `'a`.
    pub(crate) unsafe fn new(raw: *mut ffi::CParser, source: &'a str) -> Self {
        // SAFETY: caller guarantees raw is non-null and valid for 'a.
        ParseResult {
            raw: unsafe { NonNull::new_unchecked(raw) },
            source,
        }
    }

    /// Enumerate all child node IDs of a node using grammar metadata.
    ///
    /// For regular nodes, returns all node-typed fields. For list nodes,
    /// returns the list's children. Null child IDs are omitted.
    pub(crate) fn child_node_ids(&self, id: RawNodeId, grammar: &RawGrammar) -> Vec<RawNodeId> {
        let Some((ptr, tag)) = self.node_ptr(id) else {
            return vec![];
        };

        if grammar.is_list(tag) {
            // SAFETY: ptr is valid and tag confirms list layout.
            let list = unsafe { &*(ptr as *const NodeList) };
            return list
                .children()
                .iter()
                .copied()
                .filter(|id| !id.is_null())
                .collect();
        }

        let meta = grammar.field_meta(tag);
        let mut children = Vec::new();
        for field in meta {
            if field.kind == FIELD_NODE_ID {
                // SAFETY: ptr is a valid arena pointer; field.offset is a
                // codegen-computed offset within the node struct, and the
                // field at that offset is a u32 (raw node ID).
                let child_raw = unsafe {
                    let field_ptr = ptr.add(field.offset as usize) as *const u32;
                    *field_ptr
                };
                let child_id = RawNodeId(child_raw);
                if !child_id.is_null() {
                    children.push(child_id);
                }
            }
        }
        children
    }

    /// Resolve a `RawNodeId` to a typed reference, validating the tag.
    /// Returns `None` if null, invalid, or tag mismatch.
    pub(crate) fn resolve_as<T: ArenaNode>(&self, id: RawNodeId) -> Option<&'a T> {
        let (ptr, tag) = self.node_ptr(id)?;
        if tag != T::TAG {
            return None;
        }
        // SAFETY: tag matches T::TAG, confirming the arena node has type T.
        // ptr is valid for 'a. T is #[repr(C)] with a u32 tag as its first
        // field, matching the arena layout.
        Some(unsafe { &*(ptr as *const T) })
    }

    /// Resolve a `RawNodeId` as a [`NodeList`] (for list nodes).
    /// Returns `None` if null or invalid.
    pub(crate) fn resolve_list(&self, id: RawNodeId) -> Option<&'a NodeList> {
        let (ptr, _) = self.node_ptr(id)?;
        // SAFETY: ptr is valid for 'a. List nodes have NodeList layout
        // (tag, count, children[count]). The caller is responsible for
        // ensuring the id refers to a list node (enforced by codegen).
        Some(unsafe { &*(ptr as *const NodeList) })
    }

    /// Get a raw pointer to a node in the arena. Returns `(pointer, tag)`.
    pub(crate) fn node_ptr(&self, id: RawNodeId) -> Option<(*const u8, u32)> {
        if id.is_null() {
            return None;
        }
        // SAFETY: self.raw is valid for 'a. The returned pointer is
        // null-checked; all arena nodes start with a u32 tag.
        unsafe {
            let ptr = ffi::syntaqlite_parser_node(self.raw.as_ptr(), id.0);
            if ptr.is_null() {
                return None;
            }
            let tag = *ptr;
            Some((ptr as *const u8, tag))
        }
    }

    /// Return the node tag for the given ID, or `None` if null/invalid.
    pub(crate) fn node_tag(&self, id: RawNodeId) -> Option<u32> {
        self.node_ptr(id).map(|(_, tag)| tag)
    }

    /// Resolve a required node field: panics (in debug) if `id` is null,
    /// returns `Err(ErrorSpan)` if the arena node is an error placeholder,
    /// or `Err(ErrorSpan { 0, 0 })` on tag mismatch.
    pub(crate) fn required_node<T: GrammarNodeType<'a>>(
        &self,
        id: RawNodeId,
    ) -> Result<T, ErrorSpan> {
        debug_assert!(!id.is_null(), "required field has null NodeId");
        self.resolve_or_error(id)
    }

    /// Resolve an optional node field: returns `Ok(None)` if `id` is null,
    /// `Err(ErrorSpan)` if the arena node is an error placeholder, or
    /// `Ok(Some(T))` on success.
    pub(crate) fn optional_node<T: GrammarNodeType<'a>>(
        &self,
        id: RawNodeId,
    ) -> Result<Option<T>, ErrorSpan> {
        if id.is_null() {
            return Ok(None);
        }
        self.resolve_or_error(id).map(Some)
    }

    fn resolve_or_error<T: GrammarNodeType<'a>>(&self, id: RawNodeId) -> Result<T, ErrorSpan> {
        let Some((ptr, tag)) = self.node_ptr(id) else {
            return Err(ErrorSpan {
                offset: 0,
                length: 0,
            });
        };
        if tag == ffi::SYNTAQLITE_ERROR_NODE_TAG {
            // SAFETY: tag 0 guarantees CErrorNode layout ({ u32, u32, u32 }, 12 bytes).
            let e = unsafe { &*(ptr as *const ffi::CErrorNode) };
            return Err(ErrorSpan {
                offset: e.offset,
                length: e.length,
            });
        }
        T::from_arena(*self, id).ok_or(ErrorSpan {
            offset: 0,
            length: 0,
        })
    }

    /// Extract typed field values from a node using grammar metadata.
    ///
    /// Returns `(tag, fields)` where `tag` is the node's type tag and
    /// `fields` contains the extracted values. Returns `None` if the node
    /// ID is null or invalid.
    pub(crate) fn extract_fields(
        &self,
        id: RawNodeId,
        grammar: &RawGrammar,
    ) -> Option<(u32, Fields<'a>)> {
        let (ptr, tag) = self.node_ptr(id)?;
        // SAFETY: ptr is a valid arena pointer from node_ptr(); tag matches
        // the node's type, so grammar.field_meta(tag) is correct. Each field
        // pointer arithmetic is bounds-safe per the arena allocation contract.
        let meta = grammar.field_meta(tag);
        let mut fields = Fields::new();
        for m in meta {
            let val = unsafe {
                let field_ptr = ptr.add(m.offset as usize);
                match m.kind {
                    FIELD_NODE_ID => FieldVal::NodeId(RawNodeId(*(field_ptr as *const u32))),
                    FIELD_SPAN => {
                        let span = &*(field_ptr as *const SourceSpan);
                        if span.length == 0 {
                            FieldVal::Span("", 0)
                        } else {
                            FieldVal::Span(span.as_str(self.source), span.offset)
                        }
                    }
                    FIELD_BOOL => FieldVal::Bool(*(field_ptr as *const u32) != 0),
                    FIELD_FLAGS => FieldVal::Flags(*field_ptr),
                    FIELD_ENUM => FieldVal::Enum(*(field_ptr as *const u32)),
                    _ => panic!("unknown C field kind: {}", m.kind),
                }
            };
            fields.push(val);
        }
        Some((tag, fields))
    }

    /// The source text bound to this reader.
    pub(crate) fn source(&self) -> &'a str {
        self.source
    }

    /// Return all non-whitespace, non-comment token positions captured
    /// during parsing. Requires `collect_tokens: true` in [`ParserConfig`].
    pub(crate) fn tokens(&self) -> &[ffi::CTokenPos] {
        // SAFETY: raw is valid; syntaqlite_parser_tokens returns a pointer valid
        // for the lifetime of &self (until the next reset/destroy).
        unsafe { ffi_slice(self.raw.as_ptr(), ffi::syntaqlite_parser_tokens) }
    }

    /// Return all comments captured during parsing.
    /// Requires `collect_tokens: true` in [`ParserConfig`].
    pub(crate) fn comments(&self) -> &[ffi::CComment] {
        // SAFETY: raw is valid; syntaqlite_parser_comments returns a pointer valid
        // for the lifetime of &self.
        unsafe { ffi_slice(self.raw.as_ptr(), ffi::syntaqlite_parser_comments) }
    }

    /// Dump an AST node tree as indented text into `out`.
    pub(crate) fn dump_node(&self, id: RawNodeId, out: &mut String, indent: usize) {
        unsafe extern "C" {
            fn free(ptr: *mut std::ffi::c_void);
        }
        // SAFETY: raw is valid; syntaqlite_dump_node returns a malloc'd
        // NUL-terminated string (or null). We free it after copying.
        unsafe {
            let ptr = ffi::syntaqlite_dump_node(self.raw.as_ptr(), id.0, indent as u32);
            if !ptr.is_null() {
                out.push_str(&CStr::from_ptr(ptr).to_string_lossy());
                free(ptr as *mut std::ffi::c_void);
            }
        }
    }

    /// If `id` refers to a list node (per the grammar), return its child IDs.
    pub(crate) fn list_children(
        &self,
        id: RawNodeId,
        grammar: &RawGrammar,
    ) -> Option<&'a [RawNodeId]> {
        let (ptr, tag) = self.node_ptr(id)?;
        if !grammar.is_list(tag) {
            return None;
        }
        // SAFETY: ptr is a valid arena pointer and the tag confirms it is a
        // list node, so it has NodeList layout (tag, count, children[count]).
        Some(unsafe { &*(ptr as *const NodeList) }.children())
    }
}

// ── ErrorSpan ────────────────────────────────────────────────────────────────

/// A source span describing where an error node was recorded in the arena.
///
/// Returned by [`ParseResult::required_node`] and [`ParseResult::optional_node`]
/// when the resolved arena node is a [`ffi::CErrorNode`] placeholder (tag 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ErrorSpan {
    /// Byte offset of the error token in the source text.
    pub(crate) offset: u32,
    /// Byte length of the error token (0 = unknown).
    pub(crate) length: u32,
}

// ── ParserInner ───────────────────────────────────────────────────────────────

/// Holds the C parser handle and mutable state. Checked out by cursors at
/// runtime and returned on [`Drop`].
pub(crate) struct ParserInner {
    pub(crate) raw: NonNull<ffi::CParser>,
    pub(crate) source_buf: Vec<u8>,
}

impl Drop for ParserInner {
    fn drop(&mut self) {
        // SAFETY: self.raw was allocated by syntaqlite_create_parser_with_grammar
        // and has not been freed (Drop runs exactly once).
        unsafe { ffi::syntaqlite_parser_destroy(self.raw.as_ptr()) }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Copy source into `source_buf` (with null terminator) and reset the C
/// parser to begin tokenizing from the buffer.
///
/// # Safety
/// `raw` must be a valid parser pointer owned by the caller.
pub(crate) unsafe fn reset_parser(raw: *mut ffi::CParser, source_buf: &mut Vec<u8>, source: &str) {
    source_buf.clear();
    source_buf.reserve(source.len() + 1);
    source_buf.extend_from_slice(source.as_bytes());
    source_buf.push(0);

    // source_buf has at least one byte (the null terminator just pushed).
    let c_source_ptr = source_buf.as_ptr();
    // SAFETY: raw is valid (caller owns it); c_source_ptr points to
    // source_buf which is null-terminated.
    unsafe {
        ffi::syntaqlite_parser_reset(raw, c_source_ptr as *const _, source.len() as u32);
    }
}

/// Build a slice from an FFI function that returns a pointer and writes a count.
///
/// # Safety
/// `raw` must be a valid parser pointer. `f` must return a pointer that is valid
/// for the caller's borrow of the parser, and write the element count into the
/// provided `*mut u32`.
pub(crate) unsafe fn ffi_slice<'a, T>(
    raw: *mut ffi::CParser,
    f: unsafe extern "C" fn(*mut ffi::CParser, *mut u32) -> *const T,
) -> &'a [T] {
    let mut count: u32 = 0;
    let ptr = unsafe { f(raw, &mut count) };
    if count == 0 || ptr.is_null() {
        return &[];
    }
    unsafe { std::slice::from_raw_parts(ptr, count as usize) }
}

// ── ffi ───────────────────────────────────────────────────────────────────────

mod ffi {
    use std::ffi::{c_char, c_int, c_void};

    /// Opaque C parser type.
    pub(crate) enum CParser {}

    /// Mirrors C `SyntaqliteMemMethods` from `include/syntaqlite/config.h`.
    #[repr(C)]
    pub struct CMemMethods {
        pub x_malloc: unsafe extern "C" fn(usize) -> *mut c_void,
        pub x_free: unsafe extern "C" fn(*mut c_void),
    }

    /// Mirrors C `SyntaqliteParseResult` from `include/syntaqlite/parser.h`.
    #[repr(C)]
    pub(crate) struct CParseResult {
        pub(crate) root: u32,
        pub(crate) error: i32,
        pub(crate) error_msg: *const c_char,
        pub(crate) error_offset: u32,
        pub(crate) error_length: u32,
        pub(crate) saw_subquery: i32,
        pub(crate) saw_update_delete_limit: i32,
    }

    /// The kind of a comment.
    ///
    /// Mirrors C `SyntaqliteCommentKind` from `include/syntaqlite/parser.h`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    pub enum CCommentKind {
        /// A line comment starting with `--`.
        LineComment = 0,
        /// A block comment delimited by `/* ... */`.
        BlockComment = 1,
    }

    /// A comment captured during parsing, sorted by source offset.
    ///
    /// Mirrors C `SyntaqliteComment` from `include/syntaqlite/parser.h`.
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct CComment {
        pub offset: u32,
        pub length: u32,
        pub kind: CCommentKind,
    }

    /// Token flags bitfield.
    ///
    /// Mirrors C `SYNTAQLITE_TOKEN_FLAG_*` from `include/syntaqlite/parser.h`.
    pub const TOKEN_FLAG_AS_ID: u32 = 1;
    pub const TOKEN_FLAG_AS_FUNCTION: u32 = 2;
    pub const TOKEN_FLAG_AS_TYPE: u32 = 4;

    /// A non-whitespace, non-comment token position captured during parsing.
    ///
    /// Mirrors C `SyntaqliteTokenPos` from `include/syntaqlite/parser.h`.
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct CTokenPos {
        pub offset: u32,
        pub length: u32,
        /// Original token type from tokenizer (pre-fallback).
        pub type_: u32,
        /// Bitfield: `TOKEN_FLAG_AS_ID` / `AS_FUNCTION` / `AS_TYPE`.
        pub flags: u32,
    }

    /// Tag value for error placeholder nodes stored in the arena (tag 0).
    ///
    /// Mirrors C `SYNTAQLITE_ERROR_NODE_TAG` from `include/syntaqlite/parser.h`.
    pub const SYNTAQLITE_ERROR_NODE_TAG: u32 = 0;

    /// An error placeholder node in the parser arena.
    ///
    /// Mirrors C `SyntaqliteErrorNode` from `include/syntaqlite/parser.h`.
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct CErrorNode {
        pub tag: u32,
        pub offset: u32,
        pub length: u32,
    }
    use std::mem::size_of;
    const _: () = assert!(size_of::<CErrorNode>() == 12);

    unsafe extern "C" {
        // Parser lifecycle
        pub(crate) fn syntaqlite_create_parser_with_grammar(
            mem: *const CMemMethods,
            grammar: crate::grammar::ffi::CGrammar,
        ) -> *mut CParser;
        pub(crate) fn syntaqlite_parser_reset(p: *mut CParser, source: *const c_char, len: u32);
        pub(crate) fn syntaqlite_parser_next(p: *mut CParser) -> CParseResult;
        pub(crate) fn syntaqlite_parser_destroy(p: *mut CParser);

        // Parser accessors
        pub(crate) fn syntaqlite_parser_node(p: *mut CParser, node_id: u32) -> *const u32;

        // Parser configuration
        pub(crate) fn syntaqlite_parser_set_trace(p: *mut CParser, enable: c_int) -> c_int;
        pub(crate) fn syntaqlite_parser_set_collect_tokens(p: *mut CParser, enable: c_int)
        -> c_int;

        // Comments and token positions
        pub(crate) fn syntaqlite_parser_comments(
            p: *mut CParser,
            count: *mut u32,
        ) -> *const CComment;
        pub(crate) fn syntaqlite_parser_tokens(
            p: *mut CParser,
            count: *mut u32,
        ) -> *const CTokenPos;

        // AST dump
        pub(crate) fn syntaqlite_dump_node(
            p: *mut CParser,
            node_id: u32,
            indent: u32,
        ) -> *mut c_char;
    }
}

pub(crate) use ffi::{
    CComment as Comment, CCommentKind as CommentKind, CParseResult, CParser, CTokenPos as TokenPos,
};

/// Wrapper around [`ffi::syntaqlite_parser_comments`] for use outside this module.
///
/// # Safety
/// `raw` must be a valid parser pointer valid for `'a`.
pub(crate) unsafe fn raw_parser_comments<'a>(raw: *mut CParser) -> &'a [Comment] {
    // SAFETY: forwarded to ffi_slice which requires the same invariants.
    unsafe { ffi_slice(raw, ffi::syntaqlite_parser_comments) }
}

/// Wrapper around [`ffi::syntaqlite_parser_tokens`] for use outside this module.
///
/// # Safety
/// `raw` must be a valid parser pointer valid for `'a`.
pub(crate) unsafe fn raw_parser_tokens<'a>(raw: *mut CParser) -> &'a [TokenPos] {
    // SAFETY: forwarded to ffi_slice which requires the same invariants.
    unsafe { ffi_slice(raw, ffi::syntaqlite_parser_tokens) }
}

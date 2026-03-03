// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::ffi::{c_char, c_int, c_void};

use crate::dialect::ffi;

// Opaque C types
pub(crate) enum Parser {}

/// Configuration for parser construction.
#[derive(Debug, Default, Clone, Copy)]
pub struct ParserConfig {
    /// Enable parser trace output (Lemon debug trace). Default: `false`.
    pub trace: bool,
    /// Collect non-whitespace token positions during parsing. Default: `false`.
    pub collect_tokens: bool,
}

/// Holds the C parser handle and mutable state. Checked out by cursors at
/// runtime and returned on [`Drop`].
pub(crate) struct ParserInner {
    pub(crate) raw: NonNull<CParser>,
    pub(crate) source_buf: Vec<u8>,
}

impl Drop for ParserInner {
    fn drop(&mut self) {
        // SAFETY: self.raw was allocated by syntaqlite_create_parser_with_dialect
        // and has not been freed (Drop runs exactly once).
        unsafe { syntaqlite_parser_destroy(self.raw.as_ptr()) }
    }
}

/// Owns a parser instance. Reusable across inputs via `parse()`.
///
/// The parser uses an interior-mutability pattern: calling `parse()` checks
/// out the C parser state at runtime, and the returned cursor returns it on
/// drop. This allows `parse()` to take `&self` instead of `&mut self`.
pub struct Parser<'d> {
    inner: Rc<RefCell<Option<ParserInner>>>,
    /// The dialect environment used for this parser. Propagated to cursors
    /// and `NodeRef`s so consumers don't need to thread it manually.
    pub(crate) dialect: DialectEnv<'d>,
}

impl<'d> Parser<'d> {
    /// Create a parser bound to the given dialect with default configuration.
    pub fn new(dialect: impl Into<DialectEnv<'d>>) -> Self {
        Self::with_config(dialect, &ParserConfig::default())
    }

    /// Create a parser bound to the given dialect with custom configuration.
    pub fn with_config(dialect: impl Into<DialectEnv<'d>>, config: &ParserConfig) -> Self {
        let env = dialect.into();
        let ffi_env = env.to_ffi();
        // SAFETY: syntaqlite_create_parser_with_dialect(NULL, &ffi_env) allocates
        // a new parser with default malloc/free. The C side copies the env.
        let raw = NonNull::new(unsafe {
            syntaqlite_create_parser_with_dialect(std::ptr::null(), &ffi_env)
        })
        .expect("parser allocation failed");

        // SAFETY: raw is freshly created (not sealed), so these calls
        // always return 0.
        unsafe {
            syntaqlite_parser_set_trace(raw.as_ptr(), config.trace as c_int);
            syntaqlite_parser_set_collect_tokens(raw.as_ptr(), config.collect_tokens as c_int);
        }

        let inner = ParserInner {
            raw,
            source_buf: Vec::new(),
        };

        Parser {
            inner: Rc::new(RefCell::new(Some(inner))),
            dialect: env,
        }
    }

    /// Bind source text and return a `StatementCursor` for iterating
    /// statements.
    ///
    /// Copies the source into an internal buffer to add a null terminator
    /// (required by the C tokenizer). The cursor owns the copy, so the
    /// original `source` does not need to outlive the cursor.
    ///
    /// # Panics
    ///
    /// Panics if a cursor from a previous `parse()` call is still alive.
    pub fn parse(&self, source: &str) -> StatementCursor<'d> {
        let mut inner = self
            .inner
            .borrow_mut()
            .take()
            .expect("Parser::parse called while a cursor is still active");
        // SAFETY: inner.raw is valid (owned via ParserInner); source is
        // copied into source_buf which will be owned by the cursor.
        unsafe { reset_parser(inner.raw.as_ptr(), &mut inner.source_buf, source) };
        StatementCursor {
            dialect: self.dialect,
            inner: Some(inner),
            slot: Rc::clone(&self.inner),
            last_saw_subquery: false,
            last_saw_update_delete_limit: false,
        }
    }
}

/// Copy source into `source_buf` (with null terminator) and reset the C
/// parser to begin tokenizing from the buffer.
///
/// # Safety
/// `raw` must be a valid parser pointer owned by the caller.
pub(crate) unsafe fn reset_parser(raw: *mut CParser, source_buf: &mut Vec<u8>, source: &str) {
    source_buf.clear();
    source_buf.reserve(source.len() + 1);
    source_buf.extend_from_slice(source.as_bytes());
    source_buf.push(0);

    // source_buf has at least one byte (the null terminator just pushed).
    let c_source_ptr = source_buf.as_ptr();
    // SAFETY: raw is valid (caller owns it); c_source_ptr points to
    // source_buf which is null-terminated.
    unsafe {
        syntaqlite_parser_reset(raw, c_source_ptr as *const _, source.len() as u32);
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
/// On drop, the checked-out parser state is returned to the parent
/// [`Parser`].
pub struct StatementCursor<'d> {
    dialect: DialectEnv<'d>,
    /// Checked-out parser state. Returned to `slot` on drop.
    inner: Option<ParserInner>,
    /// Slot to return `inner` to when this cursor is dropped.
    slot: Rc<RefCell<Option<ParserInner>>>,
    /// Value of `saw_subquery` from the last successful `next_statement()` call.
    last_saw_subquery: bool,
    /// Value of `saw_update_delete_limit` from the last successful `next_statement()` call.
    last_saw_update_delete_limit: bool,
}

impl Drop for StatementCursor<'_> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            *self.slot.borrow_mut() = Some(inner);
        }
    }
}

impl<'d> StatementCursor<'d> {
    /// The raw C parser pointer from the checked-out inner state.
    fn raw_ptr(&self) -> *mut CParser {
        self.inner.as_ref().unwrap().raw.as_ptr()
    }

    /// Parse the next SQL statement.
    ///
    /// Returns:
    /// - `Some(Ok(node))` — successfully parsed statement root as a [`NodeRef`].
    /// - `Some(Err(e))` — syntax error for one statement; call again to
    ///   continue with subsequent statements (Lemon recovers on `;`).
    /// - `None` — all input has been consumed.
    ///
    /// The returned [`NodeRef`] borrows from the cursor. Use `while let`
    /// to iterate:
    /// ```ignore
    /// while let Some(result) = cursor.next_statement() {
    ///     let node = result?;
    ///     // use node...
    /// }
    /// ```
    pub fn next_statement(&mut self) -> Option<Result<crate::NodeRef<'_>, ParseError>> {
        // SAFETY: raw is valid and exclusively borrowed via &mut self.
        // When error is set, error_msg is a NUL-terminated string in the
        // parser's buffer (valid for parser lifetime).
        let result = unsafe { syntaqlite_parser_next(self.raw_ptr()) };

        let id = NodeId(result.root);
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
            return Some(Ok(crate::NodeRef::new(id, self.reader(), self.dialect)));
        }

        None
    }

    /// Build a [`ParseResult`] for the parser arena, borrowing source
    /// text from the internal buffer.
    ///
    /// This is lightweight (no allocation) — it packages the raw parser
    /// pointer with a `&str` view of the owned source buffer.
    pub fn reader(&self) -> ParseResult<'_> {
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

    /// The dialect environment for this cursor.
    pub fn dialect(&self) -> DialectEnv<'d> {
        self.dialect
    }

    /// Return all non-whitespace, non-comment token positions captured
    /// during parsing.
    pub fn tokens(&self) -> &[TokenPos] {
        // SAFETY: raw is valid (owned via ParserInner, valid for &self).
        unsafe {
            crate::session::ffi_slice(self.raw_ptr(), crate::parser::syntaqlite_parser_tokens)
        }
    }

    /// Return all comments captured during parsing.
    pub fn comments(&self) -> &[Comment] {
        // SAFETY: raw is valid (owned via ParserInner, valid for &self).
        unsafe {
            crate::session::ffi_slice(self.raw_ptr(), crate::parser::syntaqlite_parser_comments)
        }
    }

    /// Wrap a `NodeId` (e.g. from a `ParseError::root`) into a `NodeRef`
    /// using this cursor's reader and dialect.
    pub fn node_ref(&self, id: NodeId) -> crate::NodeRef<'_> {
        crate::NodeRef::new(id, self.reader(), self.dialect)
    }
}

/// Mirrors C `SyntaqliteParseResult` from `include/syntaqlite/parser.h`.
#[repr(C)]
pub(crate) struct ParseResult {
    pub root: u32,
    pub error: i32,
    pub error_msg: *const c_char,
    pub error_offset: u32,
    pub error_length: u32,
    pub saw_subquery: i32,
    pub saw_update_delete_limit: i32,
}

/// Mirrors C `SyntaqliteMemMethods` from `include/syntaqlite/config.h`.
#[repr(C)]
pub struct MemMethods {
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

/// Tag value for error placeholder nodes stored in the arena.
/// Tag 0 is the sentinel (NodeTag::Null = 0 in generated code) — repurposed
/// here as the error node tag without requiring codegen changes.
///
/// Mirrors C `SYNTAQLITE_ERROR_NODE_TAG` from `include/syntaqlite/parser.h`.
pub const SYNTAQLITE_ERROR_NODE_TAG: u32 = 0;

/// An error placeholder node in the parser arena.
///
/// Mirrors C `SyntaqliteErrorNode` from `include/syntaqlite/parser.h`.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ErrorNode {
    pub tag: u32,
    pub offset: u32,
    pub length: u32,
}
const _: () = assert!(std::mem::size_of::<ErrorNode>() == 12);

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
    pub(crate) fn syntaqlite_create_parser_with_dialect(
        mem: *const MemMethods,
        env: *const ffi::DialectEnv,
    ) -> *mut Parser;
    pub(crate) fn syntaqlite_parser_reset(p: *mut Parser, source: *const c_char, len: u32);
    pub(crate) fn syntaqlite_parser_next(p: *mut Parser) -> ParseResult;
    pub(crate) fn syntaqlite_parser_destroy(p: *mut Parser);

    // Parser accessors
    pub(crate) fn syntaqlite_parser_node(p: *mut Parser, node_id: u32) -> *const u32;
    pub(crate) fn syntaqlite_parser_node_count(p: *mut Parser) -> u32;

    // Parser configuration
    pub(crate) fn syntaqlite_parser_set_trace(p: *mut Parser, enable: c_int) -> c_int;
    pub(crate) fn syntaqlite_parser_set_collect_tokens(p: *mut Parser, enable: c_int) -> c_int;
    // Comments
    pub(crate) fn syntaqlite_parser_comments(p: *mut Parser, count: *mut u32) -> *const Comment;

    // Token positions
    pub(crate) fn syntaqlite_parser_tokens(p: *mut Parser, count: *mut u32) -> *const TokenPos;

    // Low-level token-feeding API
    pub(crate) fn syntaqlite_parser_feed_token(
        p: *mut Parser,
        token_type: c_int,
        text: *const c_char,
        len: c_int,
    ) -> c_int;
    pub(crate) fn syntaqlite_parser_result(p: *mut Parser) -> ParseResult;
    pub(crate) fn syntaqlite_parser_expected_tokens(
        p: *mut Parser,
        out_tokens: *mut c_int,
        out_cap: c_int,
    ) -> c_int;
    pub(crate) fn syntaqlite_parser_completion_context(p: *mut Parser) -> u32;
    pub(crate) fn syntaqlite_parser_finish(p: *mut Parser) -> c_int;

    // Macro region tracking
    pub(crate) fn syntaqlite_parser_begin_macro(p: *mut Parser, call_offset: u32, call_length: u32);
    pub(crate) fn syntaqlite_parser_end_macro(p: *mut Parser);
    pub(crate) fn syntaqlite_parser_macro_regions(
        p: *mut Parser,
        count: *mut u32,
    ) -> *const MacroRegion;

    // AST dump
    pub(crate) fn syntaqlite_dump_node(p: *mut Parser, node_id: u32, indent: u32) -> *mut c_char;

    // Tokenizer
    pub(crate) fn syntaqlite_tokenizer_create(
        mem: *const MemMethods,
        env: *const ffi::DialectEnv,
    ) -> *mut Tokenizer;
    pub(crate) fn syntaqlite_tokenizer_reset(tok: *mut Tokenizer, source: *const c_char, len: u32);
    pub(crate) fn syntaqlite_tokenizer_next(tok: *mut Tokenizer, out: *mut Token) -> c_int;
    pub(crate) fn syntaqlite_tokenizer_destroy(tok: *mut Tokenizer);

}

/// A source span describing where an error node was recorded in the arena.
///
/// Returned by [`ParseResult::required_node`] and [`ParseResult::optional_node`]
/// when the resolved arena node is an `ErrorNode` (tag 0) rather than the
/// expected typed node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorSpan {
    /// Byte offset of the error token in the source text.
    pub offset: u32,
    /// Byte length of the error token (0 = unknown).
    pub length: u32,
}

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
    /// succeeded. The tree may contain `ErrorNode` placeholders (tag 0)
    /// in positions where the parser recovered (e.g. interpolation holes).
    /// `None` when the error was unrecoverable and no tree was produced.
    pub root: Option<NodeId>,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

// ── ParseResult ──────────────────────────────────────────────────────────

/// A lightweight, `Copy` handle for reading the result of a parse: nodes,
/// tokens, comments, and macro regions from the parser arena.
///
/// This is the read-only half of a cursor state. TypedDialectEnv crates embed it in
/// view structs so that accessor methods can resolve `NodeId` children
/// without requiring a back-reference to the full cursor.
///
/// # Safety invariant
/// The raw pointer must remain valid for `'a`. This is guaranteed when
/// `ParseResult` is obtained from a cursor state (which borrows the parser
/// exclusively for `'a`).
#[derive(Clone, Copy)]
pub struct ParseResult<'a> {
    pub(crate) raw: NonNull<ffi::Parser>,
    pub(crate) source: &'a str,
}

impl<'a> ParseResult<'a> {
    /// Construct a `ParseResult` from a raw parser pointer and source text.
    ///
    /// # Safety
    /// `raw` must be a valid, non-null parser pointer that remains valid
    /// for the lifetime `'a`.
    pub(crate) unsafe fn new(raw: *mut ffi::Parser, source: &'a str) -> Self {
        // SAFETY: caller guarantees raw is non-null and valid for 'a.
        ParseResult {
            raw: unsafe { NonNull::new_unchecked(raw) },
            source,
        }
    }

    /// Enumerate all child NodeIds of a node using dialect metadata.
    ///
    /// For regular nodes, returns all `Index`-typed (child node) fields.
    /// For list nodes, returns the list's children.
    /// Null child IDs are omitted from the result.
    pub fn child_node_ids(&self, id: NodeId, dialect: &DialectEnv) -> Vec<NodeId> {
        let Some((ptr, tag)) = self.node_ptr(id) else {
            return vec![];
        };

        if dialect.is_list(tag) {
            // SAFETY: ptr is valid and tag confirms list layout.
            let list = unsafe { &*(ptr as *const NodeList) };
            return list
                .children()
                .iter()
                .copied()
                .filter(|id| !id.is_null())
                .collect();
        }

        let meta = dialect.field_meta(tag);
        let mut children = Vec::new();
        for field in meta {
            if field.kind == crate::dialect::FIELD_NODE_ID {
                // SAFETY: ptr is a valid arena pointer, field.offset is a
                // codegen-computed offset within the node struct, and the
                // field at that offset is a u32 (raw NodeId).
                let child_raw = unsafe {
                    let field_ptr = ptr.add(field.offset as usize) as *const u32;
                    *field_ptr
                };
                let child_id = NodeId(child_raw);
                if !child_id.is_null() {
                    children.push(child_id);
                }
            }
        }
        children
    }

    /// Resolve a `NodeId` to a typed reference, validating the tag matches.
    /// Returns `None` if null, invalid, or tag mismatch.
    pub fn resolve_as<T: ArenaNode>(&self, id: NodeId) -> Option<&'a T> {
        let (ptr, tag) = self.node_ptr(id)?;
        if tag != T::TAG {
            return None;
        }
        // SAFETY: tag matches T::TAG, confirming the arena node has type T.
        // ptr is valid for 'a (guaranteed by NodeReader's construction from a
        // live parser). T is #[repr(C)] with a u32 tag as its first field,
        // matching the arena layout.
        Some(unsafe { &*(ptr as *const T) })
    }

    /// Resolve a `NodeId` as a `NodeList` (for list nodes).
    /// Returns `None` if null or invalid.
    pub fn resolve_list(&self, id: NodeId) -> Option<&'a NodeList> {
        let (ptr, _) = self.node_ptr(id)?;
        // SAFETY: ptr is valid for 'a. List nodes have NodeList layout
        // (tag, count, children[count]). The caller is responsible for
        // ensuring the id refers to a list node (enforced by codegen).
        Some(unsafe { &*(ptr as *const NodeList) })
    }

    /// Get a raw pointer to a node in the arena. Returns `(pointer, tag)`.
    pub fn node_ptr(&self, id: NodeId) -> Option<(*const u8, u32)> {
        if id.is_null() {
            return None;
        }
        // SAFETY: self.raw is valid for 'a (guaranteed by NodeReader's construction
        // from a live parser). The returned pointer is null-checked; all arena nodes
        // start with a u32 tag, so the dereference is valid and aligned.
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
    pub fn node_tag(&self, id: NodeId) -> Option<u32> {
        self.node_ptr(id).map(|(_, tag)| tag)
    }

    /// Resolve a required node field: panics (in debug) if `id` is null,
    /// returns `Err(ErrorSpan)` if the arena node is an error placeholder,
    /// or `Err(ErrorSpan { 0, 0 })` if the type tag mismatches.
    pub fn required_node<T: DialectNodeType<'a>>(&self, id: NodeId) -> Result<T, ErrorSpan> {
        debug_assert!(!id.is_null(), "required field has null NodeId");
        self.resolve_or_error(id)
    }

    /// Resolve an optional node field: returns `Ok(None)` if `id` is null,
    /// `Err(ErrorSpan)` if the arena node is an error placeholder, or
    /// `Ok(Some(T))` on success.
    pub fn optional_node<T: DialectNodeType<'a>>(
        &self,
        id: NodeId,
    ) -> Result<Option<T>, ErrorSpan> {
        if id.is_null() {
            return Ok(None);
        }
        self.resolve_or_error(id).map(Some)
    }

    fn resolve_or_error<T: DialectNodeType<'a>>(&self, id: NodeId) -> Result<T, ErrorSpan> {
        let Some((ptr, tag)) = self.node_ptr(id) else {
            return Err(ErrorSpan {
                offset: 0,
                length: 0,
            });
        };
        if tag == ffi::SYNTAQLITE_ERROR_NODE_TAG {
            // SAFETY: tag 0 guarantees SyntaqliteErrorNode layout ({ u32, u32, u32 }, 12 bytes).
            let e = unsafe { &*(ptr as *const ffi::ErrorNode) };
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

    /// Extract typed field values from a node, using dialect metadata.
    ///
    /// Returns `(tag, fields)` where `tag` is the node's type tag and
    /// `fields` contains the extracted field values. Returns `None` if
    /// the node ID is null or invalid.
    pub fn extract_fields(
        &self,
        id: NodeId,
        dialect: &DialectEnv,
    ) -> Option<(u32, crate::node::Fields<'a>)> {
        let (ptr, tag) = self.node_ptr(id)?;
        // SAFETY: ptr is a valid arena pointer from node_ptr(); tag matches
        // the node's type, so dialect.field_meta(tag) is correct.
        let fields = unsafe { crate::dialect::extract_fields(dialect, ptr, tag, self.source) };
        Some((tag, fields))
    }

    /// The source text bound to this reader.
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// Return all non-whitespace, non-comment token positions captured
    /// during parsing. Requires `collect_tokens: true` in `ParserConfig`.
    pub fn tokens(&self) -> &[ffi::TokenPos] {
        // SAFETY: raw is valid; syntaqlite_parser_tokens returns a pointer valid
        // for the lifetime of &self (until the next reset/destroy, which need &mut).
        unsafe { ffi_slice(self.raw.as_ptr(), ffi::syntaqlite_parser_tokens) }
    }

    /// Return all comments captured during parsing.
    /// Requires `collect_tokens: true` in `ParserConfig`.
    pub fn comments(&self) -> &[ffi::Comment] {
        // SAFETY: raw is valid; syntaqlite_parser_comments returns a pointer valid
        // for the lifetime of &self (until the next reset/destroy, which need &mut).
        unsafe { ffi_slice(self.raw.as_ptr(), ffi::syntaqlite_parser_comments) }
    }

    /// Return all macro regions captured during parsing.
    pub fn macro_regions(&self) -> &[ffi::MacroRegion] {
        // SAFETY: raw is valid; syntaqlite_parser_macro_regions returns a pointer
        // valid for the lifetime of &self.
        unsafe { ffi_slice(self.raw.as_ptr(), ffi::syntaqlite_parser_macro_regions) }
    }

    /// Dump an AST node tree as indented text into `out`.
    pub fn dump_node(&self, id: NodeId, out: &mut String, indent: usize) {
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

    /// If `id` refers to a list node (per the dialect), return its child node IDs.
    pub fn list_children(&self, id: NodeId, dialect: &DialectEnv) -> Option<&'a [NodeId]> {
        let (ptr, tag) = self.node_ptr(id)?;
        if !dialect.is_list(tag) {
            return None;
        }
        // SAFETY: ptr is a valid arena pointer and the tag confirms it is a
        // list node, so it has NodeList layout (tag, count, children[count]).
        Some(unsafe { &*(ptr as *const NodeList) }.children())
    }
}

/// Build a slice from an FFI function that returns a pointer and writes a count.
///
/// # Safety
/// `raw` must be a valid parser pointer. `f` must return a pointer that is valid
/// for the caller's borrow of the parser, and write the element count into the
/// provided `*mut u32`.
pub(crate) unsafe fn ffi_slice<'a, T>(
    raw: *mut ffi::Parser,
    f: unsafe extern "C" fn(*mut ffi::Parser, *mut u32) -> *const T,
) -> &'a [T] {
    let mut count: u32 = 0;
    let ptr = unsafe { f(raw, &mut count) };
    if count == 0 || ptr.is_null() {
        return &[];
    }
    unsafe { std::slice::from_raw_parts(ptr, count as usize) }
}

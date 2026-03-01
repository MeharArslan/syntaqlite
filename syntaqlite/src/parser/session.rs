// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::ffi::{CStr, c_int};

use super::ffi;
use super::ffi::Comment;
use super::nodes::{NodeId, NodeList};
use crate::dialect::Dialect;
use crate::dialect::ffi::DialectConfig;

/// A source span describing where an error node was recorded in the arena.
///
/// Returned by [`NodeReader::required_node`] and [`NodeReader::optional_node`]
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

/// Owns a parser instance. Reusable across inputs via `parse()`.
pub struct BaseParser<'d> {
    pub(crate) raw: *mut ffi::Parser,
    /// Null-terminated copy of the source text. The C tokenizer (SQLite's
    /// `SynqSqliteGetToken`) reads until it hits a null byte, so we must
    /// ensure the source is null-terminated. Rust `&str` does not guarantee
    /// this. The buffer is reused across `parse()` calls to avoid repeated
    /// allocations.
    pub(crate) source_buf: Vec<u8>,
    /// Owned dialect config, kept alive so the C pointer remains valid.
    dialect_config: DialectConfig,
    /// The dialect used for this parser. Propagated to cursors and `NodeRef`s
    /// so consumers don't need to thread it manually.
    pub(crate) dialect: Dialect<'d>,
}

// SAFETY: The C parser is self-contained (no thread-local or shared mutable
// state). Moving it between threads is safe; concurrent access is prevented
// by &mut borrowing in parse().
unsafe impl Send for BaseParser<'_> {}

impl<'d> BaseParser<'d> {
    /// Create a parser for the built-in SQLite dialect with default configuration.
    #[cfg(feature = "sqlite")]
    pub fn new() -> BaseParser<'static> {
        BaseParser::builder(&crate::sqlite::DIALECT).build()
    }

    /// Create a builder for a parser bound to the given dialect.
    pub fn builder<'a>(dialect: &'a Dialect) -> BaseParserBuilder<'a> {
        BaseParserBuilder {
            dialect,
            trace: false,
            collect_tokens: false,
            dialect_config: None,
        }
    }

    /// Bind source text and return a `BaseStatementCursor` for iterating statements.
    ///
    /// Copies the source into an internal buffer to add a null terminator
    /// (required by the C tokenizer). For zero-copy parsing, use
    /// [`parse_cstr`](Self::parse_cstr).
    pub fn parse<'a>(&'a mut self, source: &'a str) -> BaseStatementCursor<'a> {
        let base = CursorBase::new(self.raw, &mut self.source_buf, source, self.dialect);
        BaseStatementCursor {
            base,
            last_saw_subquery: false,
            last_saw_update_delete_limit: false,
        }
    }

    /// Zero-copy variant: bind a null-terminated source and return a
    /// `BaseStatementCursor`.
    ///
    /// The `&CStr` already guarantees a trailing `\0`, so no copy is needed.
    /// The source must be valid UTF-8 (panics otherwise).
    pub fn parse_cstr<'a>(&'a mut self, source: &'a CStr) -> BaseStatementCursor<'a> {
        let base = CursorBase::new_cstr(self.raw, source, self.dialect);
        BaseStatementCursor {
            base,
            last_saw_subquery: false,
            last_saw_update_delete_limit: false,
        }
    }
}

#[cfg(feature = "sqlite")]
impl Default for BaseParser<'static> {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for BaseParser<'_> {
    fn drop(&mut self) {
        // SAFETY: self.raw was allocated by syntaqlite_parser_create and has
        // not been freed (Drop runs exactly once). The C function is no-op
        // on NULL.
        unsafe { ffi::syntaqlite_parser_destroy(self.raw) }
    }
}

// ── BaseParserBuilder ───────────────────────────────────────────────────────

/// Builder for configuring a [`BaseParser`] before construction.
pub struct BaseParserBuilder<'a> {
    dialect: &'a Dialect<'a>,
    trace: bool,
    collect_tokens: bool,
    dialect_config: Option<DialectConfig>,
}

impl<'a> BaseParserBuilder<'a> {
    /// Enable parser trace output (Lemon debug trace).
    pub fn trace(mut self, enable: bool) -> Self {
        self.trace = enable;
        self
    }

    /// Collect non-whitespace token positions during parsing.
    pub fn collect_tokens(mut self, enable: bool) -> Self {
        self.collect_tokens = enable;
        self
    }

    /// Set dialect config for version/cflag-gated tokenization.
    pub fn dialect_config(mut self, config: DialectConfig) -> Self {
        self.dialect_config = Some(config);
        self
    }

    /// Build the parser.
    pub fn build(self) -> BaseParser<'a> {
        // SAFETY: syntaqlite_create_parser_with_dialect(NULL, dialect) allocates
        // a new parser with default malloc/free.
        let raw = unsafe {
            ffi::syntaqlite_create_parser_with_dialect(std::ptr::null(), self.dialect.raw)
        };
        assert!(!raw.is_null(), "parser allocation failed");

        // SAFETY: raw is freshly created (not sealed), so these calls
        // always return 0.
        unsafe {
            ffi::syntaqlite_parser_set_trace(raw, self.trace as c_int);
            ffi::syntaqlite_parser_set_collect_tokens(raw, self.collect_tokens as c_int);
        }

        let mut parser = BaseParser {
            raw,
            source_buf: Vec::new(),
            dialect_config: DialectConfig::default(),
            dialect: *self.dialect,
        };

        if let Some(config) = self.dialect_config {
            parser.dialect_config = config;
            // SAFETY: We pass a pointer to parser.dialect_config which lives
            // in the BaseParser struct. The C side copies the config value.
            unsafe {
                ffi::syntaqlite_parser_set_dialect_config(
                    parser.raw,
                    &parser.dialect_config as *const DialectConfig,
                );
            }
        }

        parser
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
    /// Enumerate all child NodeIds of a node using dialect metadata.
    ///
    /// For regular nodes, returns all `Index`-typed (child node) fields.
    /// For list nodes, returns the list's children.
    /// Null child IDs are omitted from the result.
    pub fn child_node_ids(&self, id: NodeId, dialect: &crate::Dialect) -> Vec<NodeId> {
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
            if field.kind == crate::dialect::ffi::FIELD_NODE_ID {
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
    pub(crate) fn resolve_as<T: super::nodes::ArenaNode>(&self, id: NodeId) -> Option<&'a T> {
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
    pub(crate) fn resolve_list(&self, id: NodeId) -> Option<&'a NodeList> {
        let (ptr, _) = self.node_ptr(id)?;
        // SAFETY: ptr is valid for 'a. List nodes have NodeList layout
        // (tag, count, children[count]). The caller is responsible for
        // ensuring the id refers to a list node (enforced by codegen).
        Some(unsafe { &*(ptr as *const NodeList) })
    }

    /// Get a raw pointer to a node in the arena. Returns `(pointer, tag)`.
    pub(crate) fn node_ptr(&self, id: NodeId) -> Option<(*const u8, u32)> {
        if id.is_null() {
            return None;
        }
        // SAFETY: self.raw is valid for 'a (guaranteed by NodeReader's construction
        // from a live parser). The returned pointer is null-checked; all arena nodes
        // start with a u32 tag, so the dereference is valid and aligned.
        unsafe {
            let ptr = ffi::syntaqlite_parser_node(self.raw, id.0);
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
    pub fn required_node<T: crate::parser::typed_list::FromArena<'a>>(
        &self,
        id: NodeId,
    ) -> Result<T, ErrorSpan> {
        debug_assert!(!id.is_null(), "required field has null NodeId");
        self.resolve_or_error(id)
    }

    /// Resolve an optional node field: returns `Ok(None)` if `id` is null,
    /// `Err(ErrorSpan)` if the arena node is an error placeholder, or
    /// `Ok(Some(T))` on success.
    pub fn optional_node<T: crate::parser::typed_list::FromArena<'a>>(
        &self,
        id: NodeId,
    ) -> Result<Option<T>, ErrorSpan> {
        if id.is_null() {
            return Ok(None);
        }
        self.resolve_or_error(id).map(Some)
    }

    fn resolve_or_error<T: crate::parser::typed_list::FromArena<'a>>(
        &self,
        id: NodeId,
    ) -> Result<T, ErrorSpan> {
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
        // SAFETY: NodeReader<'a> is Copy and all its data (raw pointer, source
        // reference) is valid for 'a. Re-casting &self to &'a NodeReader<'a>
        // extends the borrow lifetime to 'a, which is safe because the
        // underlying parser arena lives for 'a (same pattern as resolve_as).
        let reader: &'a NodeReader<'a> = unsafe { &*(self as *const NodeReader<'a>) };
        T::from_arena(reader, id).ok_or(ErrorSpan {
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
        dialect: &crate::Dialect,
    ) -> Option<(u32, super::nodes::Fields<'a>)> {
        let (ptr, tag) = self.node_ptr(id)?;
        // SAFETY: ptr is a valid arena pointer from node_ptr(); tag matches
        // the node's type, so dialect.field_meta(tag) is correct.
        let fields = unsafe { crate::extract_fields(dialect, ptr, tag, self.source) };
        Some((tag, fields))
    }

    /// The source text bound to this reader.
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// Return all non-whitespace, non-comment token positions captured
    /// during parsing. Requires `collect_tokens: true` in `ParserConfig`.
    pub(crate) fn tokens(&self) -> &[ffi::TokenPos] {
        // SAFETY: raw is valid; syntaqlite_parser_tokens returns a pointer valid
        // for the lifetime of &self (until the next reset/destroy, which need &mut).
        unsafe { ffi_slice(self.raw, ffi::syntaqlite_parser_tokens) }
    }

    /// Return all macro regions captured during parsing.
    pub(crate) fn macro_regions(&self) -> &[super::ffi::MacroRegion] {
        // SAFETY: raw is valid; syntaqlite_parser_macro_regions returns a pointer
        // valid for the lifetime of &self.
        unsafe { ffi_slice(self.raw, ffi::syntaqlite_parser_macro_regions) }
    }

    /// Access the raw C parser pointer (crate-internal).
    pub(crate) fn raw(&self) -> *mut ffi::Parser {
        self.raw
    }

    /// If `id` refers to a list node (per the dialect), return its child node IDs.
    pub fn list_children(&self, id: NodeId, dialect: &crate::Dialect) -> Option<&'a [NodeId]> {
        let (ptr, tag) = self.node_ptr(id)?;
        if !dialect.is_list(tag) {
            return None;
        }
        // SAFETY: ptr is a valid arena pointer and the tag confirms it is a
        // list node, so it has NodeList layout (tag, count, children[count]).
        Some(unsafe { &*(ptr as *const NodeList) }.children())
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

/// Shared read-only cursor state. Both `BaseStatementCursor` and `LowLevelCursor`
/// wrap this.
pub struct CursorBase<'a> {
    pub(crate) reader: NodeReader<'a>,
    /// The pointer that the C parser uses as its source base. This may differ
    /// from `source.as_ptr()` when `parse()` copies into an internal buffer.
    /// `feed_token` translates user text pointers through this so that the C
    /// code's `tok.z - ctx->source` offset arithmetic is correct regardless
    /// of whether the copying or zero-copy path was used.
    pub(crate) c_source_ptr: *const u8,
    /// The dialect handle, propagated from the parser that created this cursor.
    pub(crate) dialect: Dialect<'a>,
}

impl<'a> CursorBase<'a> {
    /// Construct a CursorBase from a raw parser pointer and source text.
    /// Copies the source into `source_buf` to null-terminate it, then resets
    /// the C parser.
    pub(crate) fn new(
        raw: *mut ffi::Parser,
        source_buf: &'a mut Vec<u8>,
        source: &'a str,
        dialect: Dialect<'a>,
    ) -> Self {
        source_buf.clear();
        source_buf.reserve(source.len() + 1);
        source_buf.extend_from_slice(source.as_bytes());
        source_buf.push(0);

        let c_source_ptr = source_buf.as_ptr();
        // SAFETY: raw is valid (caller owns it via &mut); c_source_ptr points to
        // source_buf which is null-terminated and lives for 'a.
        unsafe {
            ffi::syntaqlite_parser_reset(raw, c_source_ptr as *const _, source.len() as u32);
        }
        CursorBase {
            reader: NodeReader { raw, source },
            c_source_ptr,
            dialect,
        }
    }

    /// Construct a CursorBase from a raw parser pointer and a CStr (zero-copy).
    pub(crate) fn new_cstr(raw: *mut ffi::Parser, source: &'a CStr, dialect: Dialect<'a>) -> Self {
        let bytes = source.to_bytes();
        let source_str = std::str::from_utf8(bytes).expect("source must be valid UTF-8");

        // SAFETY: raw is valid; source is a CStr (null-terminated, valid for 'a).
        unsafe {
            ffi::syntaqlite_parser_reset(raw, source.as_ptr(), bytes.len() as u32);
        }
        CursorBase {
            reader: NodeReader {
                raw,
                source: source_str,
            },
            c_source_ptr: source.as_ptr() as *const u8,
            dialect,
        }
    }

    /// Get a reference to the embedded `NodeReader`.
    ///
    /// The returned reference borrows `self`, so nodes resolved through it
    /// cannot outlive this cursor.
    pub fn reader(&self) -> &NodeReader<'a> {
        &self.reader
    }

    /// The source text bound to this cursor.
    pub(crate) fn source(&self) -> &'a str {
        self.reader.source()
    }

    /// Return all non-whitespace, non-comment token positions captured
    /// during parsing. Requires `collect_tokens: true` in `ParserConfig`.
    pub(crate) fn tokens(&self) -> &[ffi::TokenPos] {
        self.reader.tokens()
    }

    /// Return all comments captured during parsing.
    /// Requires `collect_tokens: true` in `ParserConfig`.
    ///
    /// Returns a slice into the parser's internal buffer — valid until
    /// the parser is reset or destroyed (which requires `&mut`).
    pub(crate) fn comments(&self) -> &[Comment] {
        // SAFETY: raw is valid; syntaqlite_parser_comments returns a pointer valid
        // for the lifetime of &self (until the next reset/destroy, which need &mut).
        unsafe { ffi_slice(self.reader.raw(), ffi::syntaqlite_parser_comments) }
    }

    /// Dump an AST node tree as indented text. Uses C-side metadata (field
    /// names, display strings) so no Rust-side string tables are needed.
    pub(crate) fn dump_node(&self, id: NodeId, out: &mut String, indent: usize) {
        self.reader.dump_node(id, out, indent)
    }
}

/// Build a slice from an FFI function that returns a pointer and writes a count.
///
/// # Safety
/// `raw` must be a valid parser pointer. `f` must return a pointer that is valid
/// for the caller's borrow of the parser, and write the element count into the
/// provided `*mut u32`.
unsafe fn ffi_slice<'a, T>(
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

// ── NodeRef ─────────────────────────────────────────────────────────────────

/// A grammar-agnostic handle to a parsed AST node.
///
/// Bundles a node's arena ID with the reader and dialect needed to
/// inspect it, enabling ergonomic methods like `name()`, `children()`,
/// and `dump_json()` without threading three arguments everywhere.
#[derive(Clone, Copy)]
pub struct NodeRef<'a> {
    id: NodeId,
    reader: NodeReader<'a>,
    dialect: Dialect<'a>,
}

impl std::fmt::Debug for NodeRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeRef").field("id", &self.id).finish()
    }
}

impl<'a> NodeRef<'a> {
    /// Create a `NodeRef` from its constituent parts.
    pub fn new(id: NodeId, reader: NodeReader<'a>, dialect: Dialect<'a>) -> Self {
        NodeRef {
            id,
            reader,
            dialect,
        }
    }

    /// Raw arena ID (escape hatch for FFI/codegen).
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Reader for typed access via `FromArena`.
    pub fn reader(&self) -> NodeReader<'a> {
        self.reader
    }

    /// Dialect handle.
    pub fn dialect(&self) -> Dialect<'a> {
        self.dialect
    }

    /// Node type tag, or `None` if null/invalid.
    pub fn tag(&self) -> Option<u32> {
        self.reader.node_tag(self.id)
    }

    /// Node type name (e.g. `"SelectStmt"`).
    pub fn name(&self) -> &str {
        match self.tag() {
            Some(tag) => self.dialect.node_name(tag),
            None => "",
        }
    }

    /// Whether this is a list node.
    pub fn is_list(&self) -> bool {
        match self.tag() {
            Some(tag) => self.dialect.is_list(tag),
            None => false,
        }
    }

    /// Child nodes: list children (for lists) or node-typed fields (for nodes).
    pub fn children(&self) -> Vec<NodeRef<'a>> {
        self.reader
            .child_node_ids(self.id, &self.dialect)
            .into_iter()
            .map(|child_id| NodeRef {
                id: child_id,
                reader: self.reader,
                dialect: self.dialect,
            })
            .collect()
    }

    /// Raw list children slice (for list nodes only).
    pub fn list_children(&self) -> Option<&'a [NodeId]> {
        self.reader.list_children(self.id, &self.dialect)
    }

    /// Dialect field metadata for this node type.
    pub fn field_meta(&self) -> &[crate::dialect::ffi::FieldMeta] {
        match self.tag() {
            Some(tag) => self.dialect.field_meta(tag),
            None => &[],
        }
    }

    /// Extract typed field values.
    pub fn extract_fields(&self) -> Option<(u32, super::nodes::Fields<'a>)> {
        self.reader.extract_fields(self.id, &self.dialect)
    }

    /// Dump as indented text (delegates to existing C-side `dump_node`).
    pub fn dump(&self, out: &mut String, indent: usize) {
        self.reader.dump_node(self.id, out, indent)
    }

    /// Dump as JSON matching the WASM AST JSON format.
    pub fn dump_json(&self, out: &mut String) {
        dump_json_id(self.id, self.reader, self.dialect, out);
    }

    /// Resolve as a typed AST node.
    pub fn as_typed<T: crate::parser::typed_list::FromArena<'a>>(&self) -> Option<T> {
        // SAFETY: NodeReader<'a> is Copy and all its data (raw pointer, source
        // reference) is valid for 'a. Re-casting to &'a NodeReader<'a> extends
        // the borrow lifetime to 'a, which is safe because the underlying
        // parser arena lives for 'a (same pattern as NodeReader::resolve_or_error).
        let reader: &'a NodeReader<'a> = unsafe { &*(&self.reader as *const NodeReader<'a>) };
        T::from_arena(reader, self.id)
    }

    /// The source text bound to this node's reader.
    pub fn source(&self) -> &'a str {
        self.reader.source()
    }
}

// ── JSON dump helpers ───────────────────────────────────────────────────────

/// Escape a string for JSON output.
fn json_escape(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < '\x20' => {
                use std::fmt::Write;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
}

/// Recursive JSON dump for a single node ID.
fn dump_json_id(id: NodeId, reader: NodeReader<'_>, dialect: Dialect<'_>, out: &mut String) {
    if id.is_null() {
        out.push_str("null");
        return;
    }
    let Some(tag) = reader.node_tag(id) else {
        out.push_str("null");
        return;
    };

    let name = dialect.node_name(tag);

    if dialect.is_list(tag) {
        let children = reader
            .list_children(id, &dialect)
            .unwrap_or(&[]);
        out.push_str("{\"type\":\"list\",\"name\":\"");
        json_escape(out, name);
        out.push_str("\",\"count\":");
        out.push_str(&children.len().to_string());
        out.push_str(",\"children\":[");
        for (i, &child_id) in children.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            if child_id.is_null() || reader.node_tag(child_id).is_none() {
                out.push_str("{\"type\":\"node\",\"name\":\"null\",\"fields\":[]}");
            } else {
                dump_json_id(child_id, reader, dialect, out);
            }
        }
        out.push_str("]}");
        return;
    }

    let meta = dialect.field_meta(tag);
    let Some((_, fields)) = reader.extract_fields(id, &dialect) else {
        out.push_str("null");
        return;
    };

    out.push_str("{\"type\":\"node\",\"name\":\"");
    json_escape(out, name);
    out.push_str("\",\"fields\":[");

    for (i, (m, fv)) in meta.iter().zip(fields.iter()).enumerate() {
        if i > 0 {
            out.push(',');
        }
        // SAFETY: m.name is a valid NUL-terminated C string from codegen.
        let label = unsafe { m.name_str() };

        match fv {
            super::nodes::FieldVal::NodeId(child_id) => {
                out.push_str("{\"kind\":\"node\",\"label\":\"");
                json_escape(out, label);
                out.push_str("\",\"child\":");
                if child_id.is_null() {
                    out.push_str("null");
                } else {
                    dump_json_id(*child_id, reader, dialect, out);
                }
                out.push('}');
            }
            super::nodes::FieldVal::Span(text, _) => {
                out.push_str("{\"kind\":\"span\",\"label\":\"");
                json_escape(out, label);
                out.push_str("\",\"value\":");
                if text.is_empty() {
                    out.push_str("null");
                } else {
                    out.push('"');
                    json_escape(out, text);
                    out.push('"');
                }
                out.push('}');
            }
            super::nodes::FieldVal::Bool(val) => {
                out.push_str("{\"kind\":\"bool\",\"label\":\"");
                json_escape(out, label);
                out.push_str("\",\"value\":");
                out.push_str(if *val { "true" } else { "false" });
                out.push('}');
            }
            super::nodes::FieldVal::Enum(val) => {
                out.push_str("{\"kind\":\"enum\",\"label\":\"");
                json_escape(out, label);
                out.push_str("\",\"value\":");
                // SAFETY: m.display is a valid C array from codegen.
                match unsafe { m.display_name(*val as usize) } {
                    Some(s) => {
                        out.push('"');
                        json_escape(out, s);
                        out.push('"');
                    }
                    None => out.push_str("null"),
                }
                out.push('}');
            }
            super::nodes::FieldVal::Flags(val) => {
                out.push_str("{\"kind\":\"flags\",\"label\":\"");
                json_escape(out, label);
                out.push_str("\",\"value\":[");
                let mut first = true;
                for bit in 0..8u8 {
                    if val & (1 << bit) != 0 {
                        if !first {
                            out.push(',');
                        }
                        first = false;
                        // SAFETY: m.display is a valid C array from codegen.
                        match unsafe { m.display_name(bit as usize) } {
                            Some(s) => {
                                out.push('"');
                                json_escape(out, s);
                                out.push('"');
                            }
                            None => {
                                out.push_str(&(1u32 << bit).to_string());
                            }
                        }
                    }
                }
                out.push_str("]}");
            }
        }
    }

    out.push_str("]}");
}

// ── BaseStatementCursor (high-level) ────────────────────────────────────────

/// A streaming cursor over parsed SQL statements. Iterate with
/// `next_statement()` or the `Iterator` impl.
///
/// On a parse error the cursor returns `Some(Err(_))` for the failing
/// statement, then continues parsing subsequent statements (Lemon's built-in
/// error recovery synchronises on `;`). Call `next_statement()` again to
/// retrieve the next valid statement.
pub struct BaseStatementCursor<'a> {
    pub(crate) base: CursorBase<'a>,
    /// Value of `saw_subquery` from the last successful `next_statement()` call.
    last_saw_subquery: bool,
    /// Value of `saw_update_delete_limit` from the last successful `next_statement()` call.
    last_saw_update_delete_limit: bool,
}

impl<'a> BaseStatementCursor<'a> {
    /// Parse the next SQL statement.
    ///
    /// Returns:
    /// - `Some(Ok(node))` — successfully parsed statement root as a [`NodeRef`].
    /// - `Some(Err(e))` — syntax error for one statement; call again to
    ///   continue with subsequent statements (Lemon recovers on `;`).
    /// - `None` — all input has been consumed.
    pub fn next_statement(&mut self) -> Option<Result<NodeRef<'a>, ParseError>> {
        // SAFETY: raw is valid and exclusively borrowed via &mut self.
        // When error is set, error_msg is a NUL-terminated string in the
        // parser's buffer (valid for parser lifetime).
        let result = unsafe { ffi::syntaqlite_parser_next(self.base.reader.raw()) };

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
            return Some(Ok(NodeRef {
                id,
                reader: self.base.reader,
                dialect: self.base.dialect,
            }));
        }

        None
    }

    /// Returns `true` if the last successfully parsed statement contained a
    /// subquery (e.g. `SELECT * FROM (SELECT 1)`, `EXISTS (SELECT ...)`,
    /// or `IN (SELECT ...)`). Reset before each statement.
    #[cfg(test)]
    pub(crate) fn saw_subquery(&self) -> bool {
        self.last_saw_subquery
    }

    /// Returns `true` if the last successfully parsed DELETE or UPDATE statement
    /// used ORDER BY or LIMIT clauses. These clauses require the
    /// `SQLITE_ENABLE_UPDATE_DELETE_LIMIT` compile-time option.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn saw_update_delete_limit(&self) -> bool {
        self.last_saw_update_delete_limit
    }

    /// Access the underlying `CursorBase` for read-only operations.
    pub(crate) fn base(&self) -> &CursorBase<'a> {
        &self.base
    }

    // Delegate read-only methods for convenience

    /// Get a reference to the embedded `NodeReader`.
    pub fn reader(&self) -> &NodeReader<'a> {
        self.base.reader()
    }

    /// The source text bound to this cursor.
    pub fn source(&self) -> &'a str {
        self.base.source()
    }

    /// Return all non-whitespace, non-comment token positions captured
    /// during parsing.
    pub fn tokens(&self) -> &[ffi::TokenPos] {
        self.base.tokens()
    }

    /// Return all comments captured during parsing.
    pub fn comments(&self) -> &[Comment] {
        self.base.comments()
    }

    /// Dump an AST node tree as indented text.
    pub fn dump_node(&self, id: NodeId, out: &mut String, indent: usize) {
        self.base.dump_node(id, out, indent)
    }

    /// Wrap a `NodeId` (e.g. from a `ParseError::root`) into a `NodeRef`
    /// using this cursor's reader and dialect.
    pub fn node_ref(&self, id: NodeId) -> NodeRef<'a> {
        NodeRef {
            id,
            reader: self.base.reader,
            dialect: self.base.dialect,
        }
    }
}

impl<'a> Iterator for BaseStatementCursor<'a> {
    type Item = Result<NodeRef<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_statement()
    }
}

#[cfg(test)]
#[cfg(feature = "sqlite")]
mod tests {
    use super::*;

    fn parse_saw_subquery(sql: &str) -> (bool, bool) {
        let mut parser = BaseParser::new();
        let mut cursor = parser.parse(sql);
        let ok = matches!(cursor.next_statement(), Some(Ok(_)));
        let saw = cursor.saw_subquery();
        (ok, saw)
    }

    #[test]
    fn node_ref_accessors() {
        let mut parser = BaseParser::new();
        let mut cursor = parser.parse("SELECT 1;");
        let node = cursor.next_statement().unwrap().unwrap();
        assert!(!node.name().is_empty());
        assert!(node.tag().is_some());
        assert!(!node.is_list());
        assert!(!node.id().is_null());
    }

    #[test]
    fn node_ref_dump_json_produces_valid_json() {
        let mut parser = BaseParser::new();
        let mut cursor = parser.parse("SELECT 1;");
        let node = cursor.next_statement().unwrap().unwrap();
        let mut out = String::new();
        node.dump_json(&mut out);
        assert!(out.starts_with("{\"type\":\"node\""));
        assert!(out.ends_with('}'));
    }

    #[test]
    fn subquery_detected_in_from() {
        let (ok, saw) = parse_saw_subquery("SELECT * FROM (SELECT 1);");
        assert!(ok, "Should parse successfully");
        assert!(saw, "Should detect subquery in FROM clause");
    }

    #[test]
    fn subquery_detected_in_exists() {
        let (ok, saw) = parse_saw_subquery("SELECT EXISTS (SELECT 1);");
        assert!(ok, "Should parse successfully");
        assert!(saw, "Should detect subquery in EXISTS expression");
    }

    #[test]
    fn subquery_detected_in_scalar_subquery() {
        let (ok, saw) = parse_saw_subquery("SELECT (SELECT 1);");
        assert!(ok, "Should parse successfully");
        assert!(saw, "Should detect scalar subquery expression");
    }

    #[test]
    fn subquery_detected_in_in_select() {
        let (ok, saw) = parse_saw_subquery("SELECT 1 WHERE 1 IN (SELECT 2);");
        assert!(ok, "Should parse successfully");
        assert!(saw, "Should detect subquery in IN (SELECT ...) expression");
    }

    #[test]
    fn no_subquery_in_simple_select() {
        let (ok, saw) = parse_saw_subquery("SELECT 1;");
        assert!(ok, "Should parse successfully");
        assert!(!saw, "Simple SELECT should NOT set saw_subquery");
    }

    #[test]
    fn no_subquery_in_in_list() {
        let (ok, saw) = parse_saw_subquery("SELECT 1 WHERE 1 IN (1, 2, 3);");
        assert!(ok, "Should parse successfully");
        assert!(!saw, "IN with literal list should NOT set saw_subquery");
    }
}

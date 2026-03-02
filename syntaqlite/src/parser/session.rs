// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::ffi::{CStr, c_int};

use crate::dialect::Dialect;
use syntaqlite_parser::dialect::ffi::DialectConfig;
use syntaqlite_parser::nodes::NodeId;
use syntaqlite_parser::parser::{
    Comment, Parser, TokenPos, syntaqlite_create_parser_with_dialect, syntaqlite_parser_comments,
    syntaqlite_parser_destroy, syntaqlite_parser_next, syntaqlite_parser_reset,
    syntaqlite_parser_set_collect_tokens, syntaqlite_parser_set_dialect_config,
    syntaqlite_parser_set_trace,
};

pub use syntaqlite_parser::session::{ErrorSpan, ParseError, RawNodeReader};

/// Owns a parser instance. Reusable across inputs via `parse()`.
pub struct RawParser<'d> {
    pub(crate) raw: *mut Parser,
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
unsafe impl Send for RawParser<'_> {}

impl<'d> RawParser<'d> {
    /// Create a parser for the built-in SQLite dialect with default configuration.
    #[cfg(feature = "sqlite")]
    pub fn new() -> RawParser<'static> {
        RawParser::builder(&crate::sqlite::DIALECT).build()
    }

    /// Create a builder for a parser bound to the given dialect.
    pub fn builder<'a>(dialect: &'a Dialect) -> RawParserBuilder<'a> {
        RawParserBuilder {
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
    pub fn parse<'a>(&'a mut self, source: &'a str) -> RawStatementCursor<'a> {
        let state = CursorState::new(self.raw, &mut self.source_buf, source, self.dialect);
        RawStatementCursor {
            state,
            last_saw_subquery: false,
            last_saw_update_delete_limit: false,
        }
    }

    /// Zero-copy variant: bind a null-terminated source and return a
    /// `BaseStatementCursor`.
    ///
    /// The `&CStr` already guarantees a trailing `\0`, so no copy is needed.
    /// The source must be valid UTF-8 (panics otherwise).
    pub fn parse_cstr<'a>(&'a mut self, source: &'a CStr) -> RawStatementCursor<'a> {
        let state = CursorState::new_cstr(self.raw, source, self.dialect);
        RawStatementCursor {
            state,
            last_saw_subquery: false,
            last_saw_update_delete_limit: false,
        }
    }
}

#[cfg(feature = "sqlite")]
impl Default for RawParser<'static> {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RawParser<'_> {
    fn drop(&mut self) {
        // SAFETY: self.raw was allocated by syntaqlite_parser_create and has
        // not been freed (Drop runs exactly once). The C function is no-op
        // on NULL.
        unsafe { syntaqlite_parser_destroy(self.raw) }
    }
}

// ── RawParserBuilder ───────────────────────────────────────────────────────

/// Builder for configuring a [`RawParser`] before construction.
pub struct RawParserBuilder<'a> {
    dialect: &'a Dialect<'a>,
    trace: bool,
    collect_tokens: bool,
    dialect_config: Option<DialectConfig>,
}

impl<'a> RawParserBuilder<'a> {
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
    pub fn build(self) -> RawParser<'a> {
        // SAFETY: syntaqlite_create_parser_with_dialect(NULL, dialect) allocates
        // a new parser with default malloc/free.
        let raw =
            unsafe { syntaqlite_create_parser_with_dialect(std::ptr::null(), self.dialect.raw) };
        assert!(!raw.is_null(), "parser allocation failed");

        // SAFETY: raw is freshly created (not sealed), so these calls
        // always return 0.
        unsafe {
            syntaqlite_parser_set_trace(raw, self.trace as c_int);
            syntaqlite_parser_set_collect_tokens(raw, self.collect_tokens as c_int);
        }

        let mut parser = RawParser {
            raw,
            source_buf: Vec::new(),
            dialect_config: DialectConfig::default(),
            dialect: *self.dialect,
        };

        if let Some(config) = self.dialect_config {
            parser.dialect_config = config;
            // SAFETY: We pass a pointer to parser.dialect_config which lives
            // in the RawParser struct. The C side copies the config value.
            unsafe {
                syntaqlite_parser_set_dialect_config(
                    parser.raw,
                    &parser.dialect_config as *const DialectConfig,
                );
            }
        }

        parser
    }
}

// ── CursorState ────────────────────────────────────────────────────────

/// Internal state shared between cursor implementations (`RawStatementCursor`,
/// `RawIncrementalCursor`). Holds the node reader, source pointer tracking,
/// and dialect handle.
pub(crate) struct CursorState<'a> {
    pub(crate) reader: RawNodeReader<'a>,
    /// The pointer that the C parser uses as its source base. This may differ
    /// from `source.as_ptr()` when `parse()` copies into an internal buffer.
    /// `feed_token` translates user text pointers through this so that the C
    /// code's `tok.z - ctx->source` offset arithmetic is correct regardless
    /// of whether the copying or zero-copy path was used.
    pub(crate) c_source_ptr: *const u8,
    /// The dialect handle, propagated from the parser that created this cursor.
    pub(crate) dialect: Dialect<'a>,
}

impl<'a> CursorState<'a> {
    /// Construct a CursorState from a raw parser pointer and source text.
    /// Copies the source into `source_buf` to null-terminate it, then resets
    /// the C parser.
    pub(crate) fn new(
        raw: *mut Parser,
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
            syntaqlite_parser_reset(raw, c_source_ptr as *const _, source.len() as u32);
        }
        CursorState {
            // SAFETY: raw is valid for 'a (caller owns it via &mut); source
            // points into source_buf which is null-terminated and lives for 'a.
            reader: unsafe { RawNodeReader::new(raw, source) },
            c_source_ptr,
            dialect,
        }
    }

    /// Construct a CursorState from a raw parser pointer and a CStr (zero-copy).
    pub(crate) fn new_cstr(raw: *mut Parser, source: &'a CStr, dialect: Dialect<'a>) -> Self {
        let bytes = source.to_bytes();
        let source_str = std::str::from_utf8(bytes).expect("source must be valid UTF-8");

        // SAFETY: raw is valid; source is a CStr (null-terminated, valid for 'a).
        unsafe {
            syntaqlite_parser_reset(raw, source.as_ptr(), bytes.len() as u32);
        }
        CursorState {
            // SAFETY: raw is valid for 'a; source_str borrows the CStr bytes
            // which live for 'a.
            reader: unsafe { RawNodeReader::new(raw, source_str) },
            c_source_ptr: source.as_ptr() as *const u8,
            dialect,
        }
    }

    /// Get a reference to the embedded `NodeReader`.
    ///
    /// The returned reference borrows `self`, so nodes resolved through it
    /// cannot outlive this cursor.
    pub(crate) fn reader(&self) -> &RawNodeReader<'a> {
        &self.reader
    }

    /// The source text bound to this cursor.
    pub(crate) fn source(&self) -> &'a str {
        self.reader.source()
    }

    /// Return all non-whitespace, non-comment token positions captured
    /// during parsing. Requires `collect_tokens: true` in `ParserConfig`.
    pub(crate) fn tokens(&self) -> &[TokenPos] {
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
        unsafe { ffi_slice(self.reader.raw(), syntaqlite_parser_comments) }
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
    raw: *mut Parser,
    f: unsafe extern "C" fn(*mut Parser, *mut u32) -> *const T,
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
    reader: RawNodeReader<'a>,
    dialect: Dialect<'a>,
}

impl std::fmt::Debug for NodeRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeRef").field("id", &self.id).finish()
    }
}

impl<'a> NodeRef<'a> {
    /// Create a `NodeRef` from its constituent parts.
    pub fn new(id: NodeId, reader: RawNodeReader<'a>, dialect: Dialect<'a>) -> Self {
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

    /// Reader for typed access via `DialectNodeType`.
    pub fn reader(&self) -> RawNodeReader<'a> {
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
    pub fn field_meta(&self) -> &[syntaqlite_parser::dialect::ffi::FieldMeta] {
        match self.tag() {
            Some(tag) => self.dialect.field_meta(tag),
            None => &[],
        }
    }

    /// Extract typed field values.
    pub fn extract_fields(&self) -> Option<(u32, syntaqlite_parser::nodes::Fields<'a>)> {
        self.reader.extract_fields(self.id, &self.dialect)
    }

    /// Dump as indented text (delegates to existing C-side `dump_node`).
    pub fn dump(&self, out: &mut String, indent: usize) {
        self.reader.dump_node(self.id, out, indent)
    }

    /// Dump as JSON matching the WASM AST JSON format.
    #[cfg(feature = "json")]
    pub fn dump_json(&self, out: &mut String) {
        let value = dump_json_id(self.id, self.reader, self.dialect);
        out.push_str(&serde_json::to_string(&value).expect("AST dump serialization failed"));
    }

    /// Resolve as a typed AST node.
    pub fn as_typed<T: syntaqlite_parser::dialect_traits::DialectNodeType<'a>>(self) -> Option<T> {
        T::from_arena(self.reader, self.id)
    }

    /// The source text bound to this node's reader.
    pub fn source(&self) -> &'a str {
        self.reader.source()
    }
}

// ── JSON dump helpers ────────────────────────────────────────────────────────

/// Recursive JSON dump for a single node ID, returning a `serde_json::Value`.
#[cfg(feature = "json")]
fn dump_json_id(id: NodeId, reader: RawNodeReader<'_>, dialect: Dialect<'_>) -> serde_json::Value {
    if id.is_null() {
        return serde_json::Value::Null;
    }
    let Some(tag) = reader.node_tag(id) else {
        return serde_json::Value::Null;
    };

    let name = dialect.node_name(tag);

    if dialect.is_list(tag) {
        let children = reader.list_children(id, &dialect).unwrap_or(&[]);
        let child_values: Vec<serde_json::Value> = children
            .iter()
            .map(|&child_id| {
                if child_id.is_null() || reader.node_tag(child_id).is_none() {
                    serde_json::json!({"type": "node", "name": "null", "fields": []})
                } else {
                    dump_json_id(child_id, reader, dialect)
                }
            })
            .collect();
        return serde_json::json!({
            "type": "list",
            "name": name,
            "count": children.len(),
            "children": child_values,
        });
    }

    let meta = dialect.field_meta(tag);
    let Some((_, fields)) = reader.extract_fields(id, &dialect) else {
        return serde_json::Value::Null;
    };

    let field_values: Vec<serde_json::Value> = meta
        .iter()
        .zip(fields.iter())
        .map(|(m, fv)| {
            // SAFETY: m.name is a valid NUL-terminated C string from codegen.
            let label = unsafe { m.name_str() };
            match fv {
                syntaqlite_parser::nodes::FieldVal::NodeId(child_id) => {
                    let child = if child_id.is_null() {
                        serde_json::Value::Null
                    } else {
                        dump_json_id(*child_id, reader, dialect)
                    };
                    serde_json::json!({"kind": "node", "label": label, "child": child})
                }
                syntaqlite_parser::nodes::FieldVal::Span(text, _) => {
                    let value = if text.is_empty() {
                        serde_json::Value::Null
                    } else {
                        serde_json::Value::String(text.to_string())
                    };
                    serde_json::json!({"kind": "span", "label": label, "value": value})
                }
                syntaqlite_parser::nodes::FieldVal::Bool(val) => {
                    serde_json::json!({"kind": "bool", "label": label, "value": val})
                }
                syntaqlite_parser::nodes::FieldVal::Enum(val) => {
                    // SAFETY: m.display is a valid C array from codegen.
                    let value = unsafe { m.display_name(*val as usize) }
                        .map(|s| serde_json::Value::String(s.to_string()))
                        .unwrap_or(serde_json::Value::Null);
                    serde_json::json!({"kind": "enum", "label": label, "value": value})
                }
                syntaqlite_parser::nodes::FieldVal::Flags(val) => {
                    let flag_values: Vec<serde_json::Value> = (0..8u8)
                        .filter(|&bit| val & (1 << bit) != 0)
                        .map(|bit| {
                            // SAFETY: m.display is a valid C array from codegen.
                            match unsafe { m.display_name(bit as usize) } {
                                Some(s) => serde_json::Value::String(s.to_string()),
                                None => serde_json::json!(1u32 << bit),
                            }
                        })
                        .collect();
                    serde_json::json!({"kind": "flags", "label": label, "value": flag_values})
                }
            }
        })
        .collect();

    serde_json::json!({
        "type": "node",
        "name": name,
        "fields": field_values,
    })
}

// ── BaseStatementCursor (high-level) ────────────────────────────────────────

/// A streaming cursor over parsed SQL statements. Iterate with
/// `next_statement()` or the `Iterator` impl.
///
/// On a parse error the cursor returns `Some(Err(_))` for the failing
/// statement, then continues parsing subsequent statements (Lemon's built-in
/// error recovery synchronises on `;`). Call `next_statement()` again to
/// retrieve the next valid statement.
pub struct RawStatementCursor<'a> {
    pub(crate) state: CursorState<'a>,
    /// Value of `saw_subquery` from the last successful `next_statement()` call.
    last_saw_subquery: bool,
    /// Value of `saw_update_delete_limit` from the last successful `next_statement()` call.
    last_saw_update_delete_limit: bool,
}

impl<'a> RawStatementCursor<'a> {
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
        let result = unsafe { syntaqlite_parser_next(self.state.reader.raw()) };

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
                reader: self.state.reader,
                dialect: self.state.dialect,
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

    /// Access the underlying `CursorState` for read-only operations.
    pub(crate) fn state(&self) -> &CursorState<'a> {
        &self.state
    }

    // Delegate read-only methods for convenience

    /// Get a reference to the embedded `NodeReader`.
    pub fn reader(&self) -> &RawNodeReader<'a> {
        self.state.reader()
    }

    /// The source text bound to this cursor.
    pub fn source(&self) -> &'a str {
        self.state.source()
    }

    /// Return all non-whitespace, non-comment token positions captured
    /// during parsing.
    pub fn tokens(&self) -> &[TokenPos] {
        self.state.tokens()
    }

    /// Return all comments captured during parsing.
    pub fn comments(&self) -> &[Comment] {
        self.state.comments()
    }

    /// Dump an AST node tree as indented text.
    pub fn dump_node(&self, id: NodeId, out: &mut String, indent: usize) {
        self.state.dump_node(id, out, indent)
    }

    /// Wrap a `NodeId` (e.g. from a `ParseError::root`) into a `NodeRef`
    /// using this cursor's reader and dialect.
    pub fn node_ref(&self, id: NodeId) -> NodeRef<'a> {
        NodeRef {
            id,
            reader: self.state.reader,
            dialect: self.state.dialect,
        }
    }
}

impl<'a> Iterator for RawStatementCursor<'a> {
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
        let mut parser = RawParser::new();
        let mut cursor = parser.parse(sql);
        let ok = matches!(cursor.next_statement(), Some(Ok(_)));
        let saw = cursor.saw_subquery();
        (ok, saw)
    }

    #[test]
    fn node_ref_accessors() {
        let mut parser = RawParser::new();
        let mut cursor = parser.parse("SELECT 1;");
        let node = cursor.next_statement().unwrap().unwrap();
        assert!(!node.name().is_empty());
        assert!(node.tag().is_some());
        assert!(!node.is_list());
        assert!(!node.id().is_null());
    }

    #[cfg(feature = "json")]
    #[test]
    fn node_ref_dump_json_produces_valid_json() {
        let mut parser = RawParser::new();
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

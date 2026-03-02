// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::ffi::{CStr, c_int};
use std::ptr::NonNull;

use crate::Dialect;
use crate::DialectConfig;
use crate::NodeId;
use crate::parser::{
    syntaqlite_create_parser_with_dialect, syntaqlite_parser_comments, syntaqlite_parser_destroy,
    syntaqlite_parser_next, syntaqlite_parser_reset, syntaqlite_parser_set_collect_tokens,
    syntaqlite_parser_set_dialect_config, syntaqlite_parser_set_trace,
};
use crate::{Comment, Parser, TokenPos};

use crate::{NodeRef, ParseError, RawNodeReader};

/// Owns a parser instance. Reusable across inputs via `parse()`.
pub struct RawParser<'d> {
    pub(crate) raw: NonNull<Parser>,
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
    /// Create a builder for a parser bound to the given dialect.
    pub fn builder(dialect: Dialect<'d>) -> RawParserBuilder<'d> {
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
        let state = CursorState::new(
            self.raw.as_ptr(),
            &mut self.source_buf,
            source,
            self.dialect,
        );
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
        let state = CursorState::new_cstr(self.raw.as_ptr(), source, self.dialect);
        RawStatementCursor {
            state,
            last_saw_subquery: false,
            last_saw_update_delete_limit: false,
        }
    }
}

impl Drop for RawParser<'_> {
    fn drop(&mut self) {
        // SAFETY: self.raw was allocated by syntaqlite_parser_create and has
        // not been freed (Drop runs exactly once).
        unsafe { syntaqlite_parser_destroy(self.raw.as_ptr()) }
    }
}

// ── RawParserBuilder ───────────────────────────────────────────────────────

/// Builder for configuring a [`RawParser`] before construction.
pub struct RawParserBuilder<'a> {
    dialect: Dialect<'a>,
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
        let raw = NonNull::new(unsafe {
            syntaqlite_create_parser_with_dialect(std::ptr::null(), self.dialect.raw)
        })
        .expect("parser allocation failed");

        // SAFETY: raw is freshly created (not sealed), so these calls
        // always return 0.
        unsafe {
            syntaqlite_parser_set_trace(raw.as_ptr(), self.trace as c_int);
            syntaqlite_parser_set_collect_tokens(raw.as_ptr(), self.collect_tokens as c_int);
        }

        let mut parser = RawParser {
            raw,
            source_buf: Vec::new(),
            dialect_config: DialectConfig::default(),
            dialect: self.dialect,
        };

        if let Some(config) = self.dialect_config {
            parser.dialect_config = config;
            // SAFETY: We pass a pointer to parser.dialect_config which lives
            // in the RawParser struct. The C side copies the config value.
            unsafe {
                syntaqlite_parser_set_dialect_config(
                    parser.raw.as_ptr(),
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
pub struct CursorState<'a> {
    pub(crate) reader: RawNodeReader<'a>,
    /// The pointer that the C parser uses as its source base. This may differ
    /// from `source.as_ptr()` when `parse()` copies into an internal buffer.
    /// `feed_token` translates user text pointers through this so that the C
    /// code's `tok.z - ctx->source` offset arithmetic is correct regardless
    /// of whether the copying or zero-copy path was used.
    pub(crate) c_source_ptr: NonNull<u8>,
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

        // source_buf has at least one byte (the null terminator just pushed).
        let c_source_ptr = NonNull::new(source_buf.as_mut_ptr()).expect("source_buf is non-empty");
        // SAFETY: raw is valid (caller owns it via &mut); c_source_ptr points to
        // source_buf which is null-terminated and lives for 'a.
        unsafe {
            syntaqlite_parser_reset(raw, c_source_ptr.as_ptr() as *const _, source.len() as u32);
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
            c_source_ptr: NonNull::new(source.as_ptr() as *mut u8).expect("CStr is non-null"),
            dialect,
        }
    }

    /// Get a reference to the embedded `NodeReader`.
    ///
    /// The returned reference borrows `self`, so nodes resolved through it
    /// cannot outlive this cursor.
    pub(crate) fn reader(&self) -> RawNodeReader<'a> {
        self.reader
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
            return Some(Ok(NodeRef::new(id, self.state.reader, self.state.dialect)));
        }

        None
    }

    /// Returns `true` if the last successfully parsed DELETE or UPDATE statement
    /// used ORDER BY or LIMIT clauses. These clauses require the
    /// `SQLITE_ENABLE_UPDATE_DELETE_LIMIT` compile-time option.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn saw_update_delete_limit(&self) -> bool {
        self.last_saw_update_delete_limit
    }

    /// Get a reference to the embedded `NodeReader`.
    pub fn reader(&self) -> RawNodeReader<'a> {
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

    /// Wrap a `NodeId` (e.g. from a `ParseError::root`) into a `NodeRef`
    /// using this cursor's reader and dialect.
    pub fn node_ref(&self, id: NodeId) -> NodeRef<'a> {
        NodeRef::new(id, self.state.reader, self.state.dialect)
    }
}

impl<'a> Iterator for RawStatementCursor<'a> {
    type Item = Result<NodeRef<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_statement()
    }
}

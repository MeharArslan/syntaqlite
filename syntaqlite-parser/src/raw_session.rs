// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::ffi::{CStr, c_int};
use std::ptr::NonNull;

use crate::DialectConfig;
use crate::NodeId;
use crate::RawDialect;
use crate::parser::{
    syntaqlite_create_parser_with_dialect, syntaqlite_parser_destroy, syntaqlite_parser_next,
    syntaqlite_parser_reset, syntaqlite_parser_set_collect_tokens,
    syntaqlite_parser_set_dialect_config, syntaqlite_parser_set_trace,
};
use crate::{Comment, Parser, TokenPos};

use crate::{NodeRef, ParseError, RawNodeReader};

/// Configuration for parser construction.
#[derive(Debug, Default, Clone, Copy)]
pub struct ParserConfig {
    /// Enable parser trace output (Lemon debug trace). Default: `false`.
    pub trace: bool,
    /// Collect non-whitespace token positions during parsing. Default: `false`.
    pub collect_tokens: bool,
    /// Dialect config for version/cflag-gated tokenization. Default: `None`.
    pub dialect_config: Option<DialectConfig>,
}

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
    pub(crate) dialect: RawDialect<'d>,
}

// SAFETY: The C parser is self-contained (no thread-local or shared mutable
// state). Moving it between threads is safe; concurrent access is prevented
// by &mut borrowing in parse().
unsafe impl Send for RawParser<'_> {}

impl<'d> RawParser<'d> {
    /// Create a parser bound to the given dialect with default configuration.
    pub fn new(dialect: impl Into<RawDialect<'d>>) -> Self {
        Self::with_config(dialect, &ParserConfig::default())
    }

    /// Create a parser bound to the given dialect with custom configuration.
    pub fn with_config(dialect: impl Into<RawDialect<'d>>, config: &ParserConfig) -> Self {
        let dialect = dialect.into();
        // SAFETY: syntaqlite_create_parser_with_dialect(NULL, dialect) allocates
        // a new parser with default malloc/free.
        let raw = NonNull::new(unsafe {
            syntaqlite_create_parser_with_dialect(std::ptr::null(), dialect.raw)
        })
        .expect("parser allocation failed");

        // SAFETY: raw is freshly created (not sealed), so these calls
        // always return 0.
        unsafe {
            syntaqlite_parser_set_trace(raw.as_ptr(), config.trace as c_int);
            syntaqlite_parser_set_collect_tokens(raw.as_ptr(), config.collect_tokens as c_int);
        }

        let mut parser = RawParser {
            raw,
            source_buf: Vec::new(),
            dialect_config: DialectConfig::default(),
            dialect,
        };

        if let Some(dc) = config.dialect_config {
            parser.dialect_config = dc;
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

    /// Bind source text and return a `BaseStatementCursor` for iterating statements.
    ///
    /// Copies the source into an internal buffer to add a null terminator
    /// (required by the C tokenizer). For zero-copy parsing, use
    /// [`parse_cstr`](Self::parse_cstr).
    pub fn parse<'a>(&'a mut self, source: &'a str) -> RawStatementCursor<'a> {
        // SAFETY: raw is valid (owned by self); source_buf lives for 'a.
        let (reader, _c_source_ptr) =
            unsafe { reset_parser(self.raw.as_ptr(), &mut self.source_buf, source) };
        RawStatementCursor {
            reader,
            dialect: self.dialect,
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
        // SAFETY: raw is valid (owned by self); source is a CStr (null-terminated, valid for 'a).
        let (reader, _c_source_ptr) = unsafe { reset_parser_cstr(self.raw.as_ptr(), source) };
        RawStatementCursor {
            reader,
            dialect: self.dialect,
            last_saw_subquery: false,
            last_saw_update_delete_limit: false,
        }
    }

    /// Get a node reader for the parser's current arena state.
    ///
    /// Valid after a `parse()` call has completed (cursor iterated to
    /// completion or dropped). The returned reader borrows the parser and
    /// the internal source buffer.
    pub fn reader(&self) -> RawNodeReader<'_> {
        let source = if self.source_buf.is_empty() {
            ""
        } else {
            // source_buf is the original source + null terminator.
            let len = self.source_buf.len() - 1;
            std::str::from_utf8(&self.source_buf[..len]).expect("source was valid UTF-8")
        };
        // SAFETY: self.raw is valid for the lifetime of &self. The source
        // borrows from source_buf which is also valid for &self.
        unsafe { RawNodeReader::new(self.raw.as_ptr(), source) }
    }
}

impl Drop for RawParser<'_> {
    fn drop(&mut self) {
        // SAFETY: self.raw was allocated by syntaqlite_parser_create and has
        // not been freed (Drop runs exactly once).
        unsafe { syntaqlite_parser_destroy(self.raw.as_ptr()) }
    }
}

/// Reset a parser with null-terminated source. Returns `(reader, c_source_ptr)`.
///
/// # Safety
/// `raw` must be a valid parser pointer owned by the caller via `&mut`.
/// `source_buf` must live for `'a`.
pub(crate) unsafe fn reset_parser<'a>(
    raw: *mut Parser,
    source_buf: &'a mut Vec<u8>,
    source: &'a str,
) -> (RawNodeReader<'a>, NonNull<u8>) {
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
    // SAFETY: raw is valid for 'a; source lives for 'a.
    let reader = unsafe { RawNodeReader::new(raw, source) };
    (reader, c_source_ptr)
}

/// Reset a parser with a CStr (zero-copy). Returns `(reader, c_source_ptr)`.
///
/// # Safety
/// `raw` must be a valid parser pointer owned by the caller via `&mut`.
pub(crate) unsafe fn reset_parser_cstr<'a>(
    raw: *mut Parser,
    source: &'a CStr,
) -> (RawNodeReader<'a>, NonNull<u8>) {
    let bytes = source.to_bytes();
    let source_str = std::str::from_utf8(bytes).expect("source must be valid UTF-8");

    // SAFETY: raw is valid; source is a CStr (null-terminated, valid for 'a).
    unsafe {
        syntaqlite_parser_reset(raw, source.as_ptr(), bytes.len() as u32);
    }
    // SAFETY: raw is valid for 'a; source_str borrows the CStr bytes
    // which live for 'a.
    let reader = unsafe { RawNodeReader::new(raw, source_str) };
    let c_source_ptr = NonNull::new(source.as_ptr() as *mut u8).expect("CStr is non-null");
    (reader, c_source_ptr)
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
    reader: RawNodeReader<'a>,
    dialect: RawDialect<'a>,
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
        let result = unsafe { syntaqlite_parser_next(self.reader.raw()) };

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
            return Some(Ok(NodeRef::new(id, self.reader, self.dialect)));
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
        self.reader
    }

    /// The source text bound to this cursor.
    pub fn source(&self) -> &'a str {
        self.reader.source()
    }

    /// Return all non-whitespace, non-comment token positions captured
    /// during parsing.
    pub fn tokens(&self) -> &[TokenPos] {
        self.reader.tokens()
    }

    /// Return all comments captured during parsing.
    pub fn comments(&self) -> &[Comment] {
        self.reader.comments()
    }

    /// Wrap a `NodeId` (e.g. from a `ParseError::root`) into a `NodeRef`
    /// using this cursor's reader and dialect.
    pub fn node_ref(&self, id: NodeId) -> NodeRef<'a> {
        NodeRef::new(id, self.reader, self.dialect)
    }
}

impl<'a> Iterator for RawStatementCursor<'a> {
    type Item = Result<NodeRef<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_statement()
    }
}

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::RefCell;
use std::ffi::{CStr, c_int};
use std::ptr::NonNull;
use std::rc::Rc;

use crate::DialectConfig;
use crate::RawDialect;
use crate::RawNodeId;
use crate::parser::{
    syntaqlite_create_parser_with_dialect, syntaqlite_parser_destroy, syntaqlite_parser_next,
    syntaqlite_parser_reset, syntaqlite_parser_set_collect_tokens,
    syntaqlite_parser_set_dialect_config, syntaqlite_parser_set_trace,
};
use crate::{Comment, Parser, TokenPos};

use crate::{NodeRef, ParseError, RawParseResult};

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

/// Holds the C parser handle and mutable state. Checked out by cursors at
/// runtime and returned on [`Drop`].
pub(crate) struct ParserInner {
    pub(crate) raw: NonNull<Parser>,
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
pub struct RawParser<'d> {
    inner: Rc<RefCell<Option<ParserInner>>>,
    /// The dialect used for this parser. Propagated to cursors and `NodeRef`s
    /// so consumers don't need to thread it manually.
    pub(crate) dialect: RawDialect<'d>,
}

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

        if let Some(dc) = config.dialect_config {
            // SAFETY: We pass a pointer to dc. The C side copies the config
            // value during this call.
            unsafe {
                syntaqlite_parser_set_dialect_config(raw.as_ptr(), &dc as *const DialectConfig);
            }
        }

        let inner = ParserInner {
            raw,
            source_buf: Vec::new(),
        };

        RawParser {
            inner: Rc::new(RefCell::new(Some(inner))),
            dialect,
        }
    }

    /// Bind source text and return a `RawStatementCursor` for iterating statements.
    ///
    /// Copies the source into an internal buffer to add a null terminator
    /// (required by the C tokenizer). For zero-copy parsing, use
    /// [`parse_cstr`](Self::parse_cstr).
    ///
    /// # Panics
    ///
    /// Panics if a cursor from a previous `parse()` call is still alive.
    pub fn parse<'a>(&self, source: &'a str) -> RawStatementCursor<'a>
    where
        'd: 'a,
    {
        let mut inner = self
            .inner
            .borrow_mut()
            .take()
            .expect("RawParser::parse called while a cursor is still active");
        // SAFETY: inner.raw is valid (owned via ParserInner); source_buf lives
        // inside inner which will be owned by the cursor for 'a.
        let (reader, _c_source_ptr) =
            unsafe { reset_parser(inner.raw.as_ptr(), &mut inner.source_buf, source) };
        RawStatementCursor {
            reader,
            dialect: self.dialect,
            inner: Some(inner),
            slot: Rc::clone(&self.inner),
            last_saw_subquery: false,
            last_saw_update_delete_limit: false,
        }
    }

    /// Zero-copy variant: bind a null-terminated source and return a
    /// `RawStatementCursor`.
    ///
    /// The `&CStr` already guarantees a trailing `\0`, so no copy is needed.
    /// The source must be valid UTF-8 (panics otherwise).
    ///
    /// # Panics
    ///
    /// Panics if a cursor from a previous `parse()` call is still alive.
    pub fn parse_cstr<'a>(&self, source: &'a CStr) -> RawStatementCursor<'a>
    where
        'd: 'a,
    {
        let inner = self
            .inner
            .borrow_mut()
            .take()
            .expect("RawParser::parse_cstr called while a cursor is still active");
        // SAFETY: inner.raw is valid (owned via ParserInner); source is a CStr
        // (null-terminated, valid for 'a).
        let (reader, _c_source_ptr) = unsafe { reset_parser_cstr(inner.raw.as_ptr(), source) };
        RawStatementCursor {
            reader,
            dialect: self.dialect,
            inner: Some(inner),
            slot: Rc::clone(&self.inner),
            last_saw_subquery: false,
            last_saw_update_delete_limit: false,
        }
    }
}

/// Reset a parser with null-terminated source. Returns `(reader, c_source_ptr)`.
///
/// # Safety
/// `raw` must be a valid parser pointer owned by the caller.
/// `source_buf` must remain valid for the lifetime of the returned `NonNull<u8>`.
pub(crate) unsafe fn reset_parser<'a>(
    raw: *mut Parser,
    source_buf: &mut Vec<u8>,
    source: &'a str,
) -> (RawParseResult<'a>, NonNull<u8>) {
    source_buf.clear();
    source_buf.reserve(source.len() + 1);
    source_buf.extend_from_slice(source.as_bytes());
    source_buf.push(0);

    // source_buf has at least one byte (the null terminator just pushed).
    let c_source_ptr = NonNull::new(source_buf.as_mut_ptr()).expect("source_buf is non-empty");
    // SAFETY: raw is valid (caller owns it); c_source_ptr points to
    // source_buf which is null-terminated.
    unsafe {
        syntaqlite_parser_reset(raw, c_source_ptr.as_ptr() as *const _, source.len() as u32);
    }
    // SAFETY: raw is valid for 'a; source lives for 'a.
    let reader = unsafe { RawParseResult::new(raw, source) };
    (reader, c_source_ptr)
}

/// Reset a parser with a CStr (zero-copy). Returns `(reader, c_source_ptr)`.
///
/// # Safety
/// `raw` must be a valid parser pointer owned by the caller.
pub(crate) unsafe fn reset_parser_cstr<'a>(
    raw: *mut Parser,
    source: &'a CStr,
) -> (RawParseResult<'a>, NonNull<u8>) {
    let bytes = source.to_bytes();
    let source_str = std::str::from_utf8(bytes).expect("source must be valid UTF-8");

    // SAFETY: raw is valid; source is a CStr (null-terminated, valid for 'a).
    unsafe {
        syntaqlite_parser_reset(raw, source.as_ptr(), bytes.len() as u32);
    }
    // SAFETY: raw is valid for 'a; source_str borrows the CStr bytes
    // which live for 'a.
    let reader = unsafe { RawParseResult::new(raw, source_str) };
    let c_source_ptr = NonNull::new(source.as_ptr() as *mut u8).expect("CStr is non-null");
    (reader, c_source_ptr)
}

// ── RawStatementCursor ──────────────────────────────────────────────────────

/// A streaming cursor over parsed SQL statements. Iterate with
/// `next_statement()` or the `Iterator` impl.
///
/// On a parse error the cursor returns `Some(Err(_))` for the failing
/// statement, then continues parsing subsequent statements (Lemon's built-in
/// error recovery synchronises on `;`). Call `next_statement()` again to
/// retrieve the next valid statement.
///
/// On drop, the checked-out parser state is returned to the parent
/// [`RawParser`].
pub struct RawStatementCursor<'a> {
    reader: RawParseResult<'a>,
    dialect: RawDialect<'a>,
    /// Checked-out parser state. Returned to `slot` on drop.
    inner: Option<ParserInner>,
    /// Slot to return `inner` to when this cursor is dropped.
    slot: Rc<RefCell<Option<ParserInner>>>,
    /// Value of `saw_subquery` from the last successful `next_statement()` call.
    last_saw_subquery: bool,
    /// Value of `saw_update_delete_limit` from the last successful `next_statement()` call.
    last_saw_update_delete_limit: bool,
}

impl Drop for RawStatementCursor<'_> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            *self.slot.borrow_mut() = Some(inner);
        }
    }
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
            return Some(Ok(NodeRef::new(id, self.reader, self.dialect)));
        }

        None
    }

    /// Get a reference to the embedded `NodeReader`.
    pub fn reader(&self) -> RawParseResult<'a> {
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

    /// Wrap a `RawNodeId` (e.g. from a `ParseError::root`) into a `NodeRef`
    /// using this cursor's reader and dialect.
    pub fn node_ref(&self, id: RawNodeId) -> NodeRef<'a> {
        NodeRef::new(id, self.reader, self.dialect)
    }
}

impl<'a> Iterator for RawStatementCursor<'a> {
    type Item = Result<NodeRef<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_statement()
    }
}

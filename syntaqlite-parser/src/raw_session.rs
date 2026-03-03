// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::RefCell;
use std::ffi::{CStr, c_int};
use std::ptr::NonNull;
use std::rc::Rc;

use crate::DialectEnv;
use crate::NodeId;
use crate::parser::Parser as CParser;
use crate::parser::{
    syntaqlite_create_parser_with_dialect, syntaqlite_parser_destroy, syntaqlite_parser_next,
    syntaqlite_parser_reset, syntaqlite_parser_set_collect_tokens, syntaqlite_parser_set_trace,
};
use crate::{Comment, TokenPos};

use crate::{ParseError, ParseResult};

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

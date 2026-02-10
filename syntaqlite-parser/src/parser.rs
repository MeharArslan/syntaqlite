use std::ffi::CStr;

use crate::ffi;
use crate::nodes::{NodeRef, NULL_NODE};

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

/// Owns a parser instance. Reusable across inputs via `parse()`.
pub struct Parser {
    raw: *mut ffi::RawParser,
}

// SAFETY: The C parser is self-contained (no thread-local or shared mutable
// state). Moving it between threads is safe; concurrent access is prevented
// by &mut borrowing in parse().
unsafe impl Send for Parser {}

impl Parser {
    pub fn new() -> Self {
        // SAFETY: syntaqlite_parser_create(NULL) allocates a new parser with
        // default malloc/free. It always succeeds (assert guards the result).
        let raw = unsafe { ffi::syntaqlite_parser_create(std::ptr::null()) };
        assert!(!raw.is_null(), "parser allocation failed");
        Parser { raw }
    }

    /// Enable Lemon trace output to stderr (debug builds only).
    pub fn set_trace(&mut self, enable: bool) {
        // SAFETY: self.raw is valid for the lifetime of Parser (Drop cleans up).
        unsafe { ffi::syntaqlite_parser_set_trace(self.raw, enable as _) }
    }

    /// Bind source text and return a `Session` for iterating statements.
    ///
    /// The session borrows `self` mutably and the source string, so both
    /// must outlive the session. Previous parse results are invalidated.
    pub fn parse<'a>(&'a mut self, source: &'a str) -> Session<'a> {
        // SAFETY: self.raw is valid. source.as_ptr() points to source.len()
        // valid bytes. The C side stores the pointer and length without copying,
        // and the borrow on `source` in Session<'a> keeps it alive.
        unsafe {
            ffi::syntaqlite_parser_reset(
                self.raw,
                source.as_ptr() as *const _,
                source.len() as u32,
            );
        }
        Session {
            parser: self,
            source,
        }
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

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

/// A parsing session tied to a source string. Iterate with `next_statement()`.
pub struct Session<'a> {
    parser: &'a mut Parser,
    source: &'a str,
}

impl<'a> Session<'a> {
    /// Parse the next SQL statement. Returns `None` when all statements have
    /// been consumed (or input was empty).
    pub fn next_statement(&mut self) -> Option<Result<u32, ParseError>> {
        // SAFETY: parser.raw is valid and exclusively borrowed via &mut self.
        let result = unsafe { ffi::syntaqlite_parser_next(self.parser.raw) };

        if result.root != NULL_NODE {
            return Some(Ok(result.root));
        }

        if result.error != 0 {
            // SAFETY: When error is set, error_msg points to a NUL-terminated
            // string in the parser's error_msg buffer (valid for parser lifetime).
            let msg = unsafe { CStr::from_ptr(result.error_msg) }
                .to_string_lossy()
                .into_owned();
            return Some(Err(ParseError { message: msg }));
        }

        None
    }

    /// Look up a node by its arena ID.
    pub fn node(&self, id: u32) -> Option<NodeRef<'a>> {
        if id == NULL_NODE {
            return None;
        }
        // SAFETY: parser.raw is valid. syntaqlite_parser_node returns a pointer
        // into the arena that is valid until the next reset() or destroy(), both
        // of which require &mut access that Session holds exclusively. The 'a
        // lifetime on NodeRef is bounded by the Session borrow.
        let ptr = unsafe { ffi::syntaqlite_parser_node(self.parser.raw, id) };
        Some(unsafe { NodeRef::from_raw(ptr) })
    }

    /// The source text bound to this session.
    pub fn source(&self) -> &'a str {
        self.source
    }
}

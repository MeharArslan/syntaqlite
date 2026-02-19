use std::ffi::CStr;

use syntaqlite_runtime::Session;

use crate::dialect::sqlite_dialect;

/// A SQLite SQL parser. Wraps the runtime parser with the SQLite dialect
/// baked in, so callers never need to think about dialects.
pub struct Parser {
    inner: syntaqlite_runtime::Parser,
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            inner: syntaqlite_runtime::Parser::new(sqlite_dialect()),
        }
    }

    /// Enable Lemon trace output to stderr (debug builds only).
    pub fn set_trace(&mut self, enable: bool) {
        self.inner.set_trace(enable);
    }

    /// Enable token/trivia collection. Required for comment preservation
    /// during formatting.
    pub fn set_collect_tokens(&mut self, enable: bool) {
        self.inner.set_collect_tokens(enable);
    }

    /// Bind source text and return a `Session` for iterating statements.
    pub fn parse<'a>(&'a mut self, source: &'a str) -> Session<'a> {
        self.inner.parse(source)
    }

    /// Zero-copy variant: bind a null-terminated source and return a `Session`.
    pub fn parse_cstr<'a>(&'a mut self, source: &'a CStr) -> Session<'a> {
        self.inner.parse_cstr(source)
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

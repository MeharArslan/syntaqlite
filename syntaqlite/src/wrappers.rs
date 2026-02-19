use std::ops::Range;

use crate::generated::nodes::Node;
use crate::generated::tokens::TokenType;

// ── Parser ──────────────────────────────────────────────────────────────

/// A parser pre-configured for the SQLite dialect.
///
/// Returns typed `StatementCursor` wrappers from `parse()`.
pub struct Parser {
    inner: syntaqlite_runtime::Parser,
}

impl Parser {
    /// Create a new parser for the SQLite dialect with default configuration.
    pub fn new() -> Self {
        Parser {
            inner: syntaqlite_runtime::Parser::new(crate::low_level::Sqlite::dialect()),
        }
    }

    /// Enable parser trace output (prints state transitions to stderr).
    pub fn with_trace(mut self) -> Self {
        self.inner.set_trace(true);
        self
    }

    /// Parse source text and return a `StatementCursor` for iterating statements.
    pub fn parse<'a>(&'a mut self, source: &'a str) -> StatementCursor<'a> {
        StatementCursor { inner: self.inner.parse(source) }
    }
}

// ── StatementCursor ─────────────────────────────────────────────────────

/// A high-level parsing cursor with typed SQLite node access.
pub struct StatementCursor<'a> {
    inner: syntaqlite_runtime::StatementCursor<'a>,
}

impl<'a> StatementCursor<'a> {
    /// Parse the next SQL statement.
    pub fn next_statement(
        &mut self,
    ) -> Option<Result<syntaqlite_runtime::NodeId, syntaqlite_runtime::ParseError>> {
        self.inner.next_statement()
    }

    /// Get a typed AST node by ID.
    pub fn node(&self, id: syntaqlite_runtime::NodeId) -> Option<Node<'a>> {
        let (ptr, _tag) = self.inner.node_ptr(id)?;
        Some(unsafe { Node::from_raw(ptr as *const u32) })
    }

    /// The source text bound to this cursor.
    pub fn source(&self) -> &'a str {
        self.inner.source()
    }

    /// Return all trivia (comments) captured during parsing.
    pub fn trivia(&self) -> &[syntaqlite_runtime::Trivia] {
        self.inner.trivia()
    }

    /// Dump an AST node tree as indented text.
    pub fn dump_node(
        &self,
        id: syntaqlite_runtime::NodeId,
        out: &mut String,
        indent: usize,
    ) {
        self.inner.dump_node(id, out, indent)
    }

    /// Return all macro regions.
    pub fn macro_regions(&self) -> &[syntaqlite_runtime::MacroRegion] {
        self.inner.macro_regions()
    }

    /// Access the underlying `CursorBase` (e.g. for `Formatter::format_node`).
    pub fn base(&self) -> &syntaqlite_runtime::CursorBase<'a> {
        self.inner.base()
    }
}

impl Iterator for StatementCursor<'_> {
    type Item = Result<syntaqlite_runtime::NodeId, syntaqlite_runtime::ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_statement()
    }
}

// ── TokenParser ─────────────────────────────────────────────────────────

/// A low-level token parser pre-configured for the SQLite dialect.
pub struct TokenParser {
    inner: syntaqlite_runtime::TokenParser,
}

impl TokenParser {
    /// Create a new token parser for the SQLite dialect.
    pub fn new() -> Self {
        TokenParser {
            inner: syntaqlite_runtime::TokenParser::new(crate::low_level::Sqlite::dialect()),
        }
    }

    /// Enable parser trace output (prints state transitions to stderr).
    pub fn with_trace(mut self) -> Self {
        self.inner.set_trace(true);
        self
    }

    /// Enable token collection (needed for trivia/comment capture).
    pub fn with_collect_tokens(mut self) -> Self {
        self.inner.set_collect_tokens(true);
        self
    }

    /// Bind source text and return a `TokenFeeder` for low-level token feeding.
    pub fn feed<'a>(&'a mut self, source: &'a str) -> TokenFeeder<'a> {
        TokenFeeder { inner: self.inner.feed(source) }
    }
}

// ── TokenFeeder ─────────────────────────────────────────────────────────

/// A low-level token-feeding cursor with typed SQLite node/token access.
pub struct TokenFeeder<'a> {
    inner: syntaqlite_runtime::TokenFeeder<'a>,
}

impl<'a> TokenFeeder<'a> {
    /// Feed a typed token to the parser.
    ///
    /// `span` is a byte range into the source text bound by this feeder.
    pub fn feed_token(
        &mut self,
        token_type: TokenType,
        span: Range<usize>,
    ) -> Result<Option<syntaqlite_runtime::NodeId>, syntaqlite_runtime::ParseError> {
        self.inner.feed_token(token_type.into(), span)
    }

    /// Signal end of input.
    pub fn finish(
        &mut self,
    ) -> Result<Option<syntaqlite_runtime::NodeId>, syntaqlite_runtime::ParseError> {
        self.inner.finish()
    }

    /// Mark subsequent fed tokens as being inside a macro expansion.
    pub fn begin_macro(&mut self, call_offset: u32, call_length: u32) {
        self.inner.begin_macro(call_offset, call_length)
    }

    /// End the innermost macro expansion region.
    pub fn end_macro(&mut self) {
        self.inner.end_macro()
    }

    /// Get a typed AST node by ID.
    pub fn node(&self, id: syntaqlite_runtime::NodeId) -> Option<Node<'a>> {
        let (ptr, _tag) = self.inner.node_ptr(id)?;
        Some(unsafe { Node::from_raw(ptr as *const u32) })
    }

    /// The source text bound to this feeder.
    pub fn source(&self) -> &'a str {
        self.inner.source()
    }

    /// Return all trivia (comments) captured during parsing.
    pub fn trivia(&self) -> &[syntaqlite_runtime::Trivia] {
        self.inner.trivia()
    }

    /// Dump an AST node tree as indented text.
    pub fn dump_node(
        &self,
        id: syntaqlite_runtime::NodeId,
        out: &mut String,
        indent: usize,
    ) {
        self.inner.dump_node(id, out, indent)
    }

    /// Return all macro regions.
    pub fn macro_regions(&self) -> &[syntaqlite_runtime::MacroRegion] {
        self.inner.macro_regions()
    }

    /// Access the underlying `CursorBase` (e.g. for `Formatter::format_node`).
    pub fn base(&self) -> &syntaqlite_runtime::CursorBase<'a> {
        self.inner.base()
    }
}

// ── Formatter ───────────────────────────────────────────────────────────

/// SQL formatter pre-configured for the SQLite dialect.
pub struct Formatter {
    inner: syntaqlite_runtime::fmt::Formatter<'static>,
}

impl Formatter {
    /// Create a formatter with default configuration.
    pub fn new() -> Result<Self, &'static str> {
        let inner = syntaqlite_runtime::fmt::Formatter::new(crate::low_level::Sqlite::dialect())?;
        Ok(Formatter { inner })
    }

    /// Set the format configuration.
    pub fn with_config(mut self, config: syntaqlite_runtime::fmt::FormatConfig) -> Self {
        self.inner = self.inner.with_config(config);
        self
    }

    /// Set whether to append semicolons after each statement.
    pub fn with_semicolons(mut self, semicolons: bool) -> Self {
        self.inner = self.inner.with_semicolons(semicolons);
        self
    }

    /// Access the current configuration.
    pub fn config(&self) -> &syntaqlite_runtime::fmt::FormatConfig {
        self.inner.config()
    }

    /// Format SQL source text.
    pub fn format(
        &mut self,
        source: &str,
    ) -> Result<String, syntaqlite_runtime::ParseError> {
        self.inner.format(source)
    }

    /// Format a single pre-parsed AST node.
    pub fn format_node(
        &self,
        cursor: &StatementCursor<'_>,
        node_id: syntaqlite_runtime::NodeId,
    ) -> String {
        self.inner.format_node(cursor.base(), node_id)
    }

}

// ── Tokenizer ───────────────────────────────────────────────────────────

/// A tokenizer for SQLite SQL.
pub struct Tokenizer {
    inner: syntaqlite_runtime::Tokenizer,
}

impl Tokenizer {
    /// Create a new tokenizer.
    pub fn new() -> Self {
        Tokenizer {
            inner: syntaqlite_runtime::Tokenizer::new(),
        }
    }

    /// Bind source text and return a cursor for iterating typed tokens.
    pub fn tokenize<'a>(&'a mut self, source: &'a str) -> TokenCursor<'a> {
        TokenCursor {
            inner: self.inner.tokenize(source),
        }
    }

    /// Zero-copy variant: bind a null-terminated source and return a
    /// `TokenCursor`. The source must be valid UTF-8 (panics otherwise).
    pub fn tokenize_cstr<'a>(&'a mut self, source: &'a std::ffi::CStr) -> TokenCursor<'a> {
        TokenCursor {
            inner: self.inner.tokenize_cstr(source),
        }
    }
}

// ── TokenCursor ────────────────────────────────────────────────────

/// An active tokenizer session yielding typed SQLite tokens.
pub struct TokenCursor<'a> {
    inner: syntaqlite_runtime::TokenCursor<'a>,
}

impl<'a> Iterator for TokenCursor<'a> {
    type Item = (TokenType, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let raw = self.inner.next()?;
        let tt = TokenType::from_raw(raw.token_type)
            .unwrap_or(TokenType::Illegal);
        Some((tt, raw.text))
    }
}

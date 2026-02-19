use crate::generated::nodes::Node;
use crate::generated::tokens::TokenType;

// ── Parser ──────────────────────────────────────────────────────────────

/// A parser pre-configured for the SQLite dialect.
///
/// Returns typed `Session` / `TokenSession` wrappers from `parse()` and
/// `token_session()`.
pub struct Parser {
    inner: syntaqlite_runtime::Parser,
}

impl Parser {
    /// Create a new parser for the SQLite dialect.
    pub fn new() -> Self {
        Parser {
            inner: syntaqlite_runtime::Parser::new(crate::dialect()),
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

    /// Parse source text and return a `Session` for iterating statements.
    pub fn parse<'a>(&'a mut self, source: &'a str) -> Session<'a> {
        Session { inner: self.inner.parse(source) }
    }

    /// Bind source text and return a `TokenSession` for low-level token feeding.
    pub fn token_session<'a>(&'a mut self, source: &'a str) -> TokenSession<'a> {
        TokenSession { inner: self.inner.token_session(source) }
    }
}

// ── Session ─────────────────────────────────────────────────────────────

/// A high-level parsing session with typed SQLite node access.
pub struct Session<'a> {
    inner: syntaqlite_runtime::Session<'a>,
}

impl<'a> Session<'a> {
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

    /// The source text bound to this session.
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

    /// Access the underlying `SessionBase` (e.g. for `Formatter::format_node`).
    pub fn base(&self) -> &syntaqlite_runtime::SessionBase<'a> {
        self.inner.base()
    }
}

impl Iterator for Session<'_> {
    type Item = Result<syntaqlite_runtime::NodeId, syntaqlite_runtime::ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_statement()
    }
}

// ── TokenSession ────────────────────────────────────────────────────────

/// A low-level token-feeding session with typed SQLite node/token access.
pub struct TokenSession<'a> {
    inner: syntaqlite_runtime::TokenSession<'a>,
}

impl<'a> TokenSession<'a> {
    /// Feed a typed token to the parser.
    pub fn feed(
        &mut self,
        token_type: TokenType,
        text: &str,
    ) -> Result<Option<syntaqlite_runtime::NodeId>, syntaqlite_runtime::ParseError> {
        self.inner.feed_token(token_type.into(), text)
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

    /// The source text bound to this session.
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

    /// Access the underlying `SessionBase` (e.g. for `Formatter::format_node`).
    pub fn base(&self) -> &syntaqlite_runtime::SessionBase<'a> {
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
        let inner = syntaqlite_runtime::fmt::Formatter::new(crate::dialect())?;
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
        session: &syntaqlite_runtime::SessionBase<'_>,
        node_id: syntaqlite_runtime::NodeId,
    ) -> String {
        self.inner.format_node(session, node_id)
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

    /// Bind source text and return a session for iterating typed tokens.
    pub fn tokenize<'a>(&'a mut self, source: &'a str) -> TokenizerSession<'a> {
        TokenizerSession {
            inner: self.inner.tokenize(source),
        }
    }
}

// ── TokenizerSession ────────────────────────────────────────────────────

/// An active tokenizer session yielding typed SQLite tokens.
pub struct TokenizerSession<'a> {
    inner: syntaqlite_runtime::TokenizerSession<'a>,
}

impl<'a> Iterator for TokenizerSession<'a> {
    type Item = (TokenType, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let raw = self.inner.next()?;
        let tt = TokenType::from_raw(raw.token_type)
            .unwrap_or(TokenType::Illegal);
        Some((tt, raw.text))
    }
}

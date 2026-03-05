// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::RefCell;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::rc::Rc;

use crate::any::{AnyNodeTag, AnyTokenType};
use crate::ast::{AnyNode, AnyNodeId, ArenaNode, GrammarNodeType, GrammarTokenType, RawNodeList};
use crate::grammar::{AnyGrammar, TypedGrammar};

// ── Public API ───────────────────────────────────────────────────────────────

/// Configuration for parser construction.
#[derive(Debug, Default, Clone, Copy)]
pub struct ParserConfig {
    trace: bool,
    collect_tokens: bool,
}

impl ParserConfig {
    /// Enable parser trace output (Lemon debug trace). Default: `false`.
    pub fn trace(&self) -> bool {
        self.trace
    }

    /// Collect non-whitespace token positions during parsing. Default: `false`.
    pub fn collect_tokens(&self) -> bool {
        self.collect_tokens
    }

    /// Set whether to enable parser trace output.
    #[must_use]
    pub fn with_trace(mut self, trace: bool) -> Self {
        self.trace = trace;
        self
    }

    /// Set whether to collect non-whitespace token positions.
    #[must_use]
    pub fn with_collect_tokens(mut self, collect_tokens: bool) -> Self {
        self.collect_tokens = collect_tokens;
        self
    }
}

/// A parser for the `SQLite` dialect. Yields [`ParseSession`]s with
/// SQLite-specific node types. For other dialects use [`TypedParser`] directly;
/// for grammar-agnostic use with raw grammars use [`AnyParser`].
///
/// Owns a parser instance. Reusable across inputs via `parse()` and
/// `incremental_parse()`. Uses an interior-mutability checkout pattern so both
/// methods take `&self` rather than `&mut self`.
#[cfg(feature = "sqlite")]
#[doc(hidden)]
pub struct Parser(TypedParser<crate::sqlite::grammar::Grammar>);

#[cfg(feature = "sqlite")]
impl Parser {
    /// Create a parser for the `SQLite` dialect with default configuration.
    pub fn new() -> Self {
        Parser(TypedParser::new(crate::sqlite::grammar::grammar()))
    }

    /// Create a parser for the `SQLite` dialect with custom configuration.
    pub fn with_config(config: &ParserConfig) -> Self {
        Parser(TypedParser::with_config(
            crate::sqlite::grammar::grammar(),
            config,
        ))
    }

    /// Bind source text and return a [`ParseSession`] for iterating statements.
    ///
    /// # Panics
    ///
    /// Panics if a session from a previous `parse()` or `incremental_parse()`
    /// call is still alive.
    pub fn parse(&self, source: &str) -> ParseSession {
        ParseSession(self.0.parse(source))
    }

    /// Bind source text and return an
    /// [`IncrementalParseSession`](crate::IncrementalParseSession)
    /// for token-by-token feeding.
    ///
    /// # Panics
    ///
    /// Panics if a session from a previous `parse()` or `incremental_parse()`
    /// call is still alive.
    pub fn incremental_parse(&self, source: &str) -> IncrementalParseSession {
        self.0.incremental_parse(source).into()
    }
}

#[cfg(feature = "sqlite")]
impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

/// An active session over parsed `SQLite` statements. Produced by [`Parser::parse`].
///
/// On a parse error the session returns `Some(Err(ParseError))` for the
/// failing statement, then continues parsing subsequent statements (Lemon's
/// built-in error recovery synchronises on `;`). Call [`next`](Self::next)
/// again to retrieve the next valid statement.
///
#[cfg(feature = "sqlite")]
#[doc(hidden)]
pub struct ParseSession(TypedParseSession<crate::sqlite::grammar::Grammar>);

/// Parse error classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ParseErrorKind {
    /// Error recovery succeeded and produced a partial tree.
    Recovered = 1,
    /// Unrecoverable parse failure.
    Fatal = 2,
}

impl ParseErrorKind {
    fn from_raw(v: u32) -> Self {
        match v {
            1 => Self::Recovered,
            _ => Self::Fatal,
        }
    }
}

#[cfg(feature = "sqlite")]
impl ParseSession {
    /// Parse the next SQL statement.
    ///
    /// Returns:
    /// - `Some(Ok(_))` — successfully parsed statement; use
    ///   [`ParsedStatement::root`] to access the typed AST,
    ///   [`ParsedStatement::tokens`] / [`ParsedStatement::comments`] for
    ///   per-statement token data.
    /// - `Some(Err(_))` — syntax error; the error's
    ///   [`root()`](ParseError::root) may contain a partially recovered tree.
    ///   Use [`ParseError::kind`] to distinguish recovered vs fatal.
    ///   Call again to continue with subsequent statements (Lemon recovers
    ///   on `;`).
    /// - `None` — all input has been consumed.
    pub fn next(&mut self) -> Option<Result<ParsedStatement<'_>, ParseError<'_>>> {
        Some(match self.0.next()? {
            Ok(result) => Ok(ParsedStatement(result)),
            Err(err) => Err(ParseError(err)),
        })
    }

    /// The source text bound to this session.
    pub fn source(&self) -> &str {
        self.0.source()
    }

    /// Get an [`AnyParsedStatement`] view of the arena after exhausting all statements.
    pub fn arena_result(&self) -> AnyParsedStatement<'_> {
        self.0.arena_result()
    }
}

/// A recorded `SQLite` token position from a parsed statement.
///
/// Returned by [`ParsedStatement::tokens`]. Requires `collect_tokens: true`
/// in [`ParserConfig`]. This is the `SQLite`-specific form of the
/// grammar-generic [`TypedParserToken`].
#[cfg(feature = "sqlite")]
pub struct ParserToken<'a>(TypedParserToken<'a, crate::sqlite::grammar::Grammar>);

#[cfg(feature = "sqlite")]
impl<'a> ParserToken<'a> {
    /// The source text slice covered by this token.
    pub fn text(&self) -> &'a str {
        self.0.text()
    }

    /// The `SQLite` token type.
    pub fn token_type(&self) -> crate::sqlite::tokens::TokenType {
        self.0.token_type()
    }

    /// Token-usage flags set during disambiguation.
    pub fn flags(&self) -> ParserTokenFlags {
        self.0.flags()
    }
}

#[cfg(feature = "sqlite")]
impl std::fmt::Debug for ParserToken<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParserToken")
            .field("text", &self.0.text())
            .field("token_type", &self.0.token_type())
            .field("flags", &self.0.flags())
            .finish()
    }
}

/// The result of a successfully parsed `SQLite` statement.
/// Produced by [`ParseSession::next`].
#[cfg(feature = "sqlite")]
#[doc(hidden)]
pub struct ParsedStatement<'a>(
    pub(crate) TypedParsedStatement<'a, crate::sqlite::grammar::Grammar>,
);

#[cfg(feature = "sqlite")]
impl<'a> ParsedStatement<'a> {
    /// The typed root node for this statement, or `None` if unavailable.
    pub fn root(&self) -> Option<crate::sqlite::ast::Stmt<'a>> {
        self.0.root()
    }

    /// The source text bound to this result.
    pub fn source(&self) -> &'a str {
        self.0.source()
    }

    /// Per-statement token positions. Requires `collect_tokens: true`.
    pub fn tokens(&self) -> impl Iterator<Item = ParserToken<'a>> {
        self.0.tokens().map(ParserToken)
    }

    /// Per-statement comments. Requires `collect_tokens: true`.
    pub fn comments(&self) -> impl Iterator<Item = Comment<'a>> {
        self.0.comments()
    }

    /// Erase the grammar type parameter, returning a type-erased [`AnyParsedStatement`].
    ///
    /// Use this to access grammar-agnostic APIs such as [`AnyParsedStatement::extract_fields`],
    /// [`AnyParsedStatement::list_children`], and [`AnyParsedStatement::macro_regions`].
    pub fn erase(&self) -> AnyParsedStatement<'a> {
        self.0.erase()
    }
}

/// A parse error from the `SQLite` dialect parser.
///
/// Obtain via [`ParseSession::next`].
#[cfg(feature = "sqlite")]
#[doc(hidden)]
pub struct ParseError<'a>(pub(crate) TypedParseError<'a, crate::sqlite::grammar::Grammar>);

#[cfg(feature = "sqlite")]
impl<'a> ParseError<'a> {
    /// Returns whether the error is recovered or fatal.
    pub fn kind(&self) -> ParseErrorKind {
        self.0.kind()
    }

    /// True if this error was recovered and yielded a partial tree.
    pub fn is_recovered(&self) -> bool {
        self.0.is_recovered()
    }

    /// True if this error is fatal (unrecoverable).
    pub fn is_fatal(&self) -> bool {
        self.0.is_fatal()
    }

    /// Returns the human-readable error message.
    pub fn message(&self) -> &str {
        self.0.message()
    }
    /// Returns the byte offset of the error token, or `None` if unknown.
    pub fn offset(&self) -> Option<usize> {
        self.0.offset()
    }
    /// Returns the byte length of the error token, or `None` if unknown.
    pub fn length(&self) -> Option<usize> {
        self.0.length()
    }
    /// Returns the partial recovery tree produced by error recovery, if any.
    pub fn root(&self) -> Option<crate::sqlite::ast::Stmt<'a>> {
        self.0.root()
    }
}

#[cfg(feature = "sqlite")]
impl std::fmt::Debug for ParseError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(feature = "sqlite")]
impl std::fmt::Display for ParseError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(feature = "sqlite")]
impl std::error::Error for ParseError<'_> {}

/// A type-safe parser scoped to a specific dialect `G`.
///
/// Yields [`TypedParseSession<G>`]s. For the common `SQLite` case use
/// [`Parser`]. For grammar-agnostic use with raw grammars use [`AnyParser`].
///
pub struct TypedParser<G: TypedGrammar> {
    inner: Rc<RefCell<Option<ParserInner>>>,
    grammar: AnyGrammar,
    _marker: PhantomData<G>,
}

impl<G: TypedGrammar> TypedParser<G> {
    /// Create a parser bound to the given dialect grammar with default configuration.
    pub fn new(grammar: G) -> Self {
        Self::with_config(grammar, &ParserConfig::default())
    }

    /// Create a parser bound to the given dialect grammar with custom configuration.
    ///
    /// # Panics
    /// Panics if the underlying C parser allocation fails (out of memory).
    pub fn with_config(grammar: G, config: &ParserConfig) -> Self {
        let grammar_raw: AnyGrammar = grammar.into();
        // SAFETY: create(NULL, grammar_raw.inner) allocates a new parser with
        // default malloc/free. The C side copies the grammar.
        let mut raw = NonNull::new(unsafe { CParser::create(std::ptr::null(), grammar_raw.inner) })
            .expect("parser allocation failed");

        // SAFETY: raw is freshly created (not sealed), so these calls always return 0.
        unsafe {
            raw.as_mut().set_trace(u32::from(config.trace()));
            raw.as_mut()
                .set_collect_tokens(u32::from(config.collect_tokens()));
        }

        TypedParser {
            inner: Rc::new(RefCell::new(Some(ParserInner {
                raw,
                source_buf: Vec::new(),
            }))),
            grammar: grammar_raw,
            _marker: PhantomData,
        }
    }

    /// Bind source text and return a [`TypedParseSession`] for iterating statements.
    ///
    /// # Panics
    ///
    /// Panics if a session from a previous `parse()` or `incremental_parse()`
    /// call is still alive.
    pub fn parse(&self, source: &str) -> TypedParseSession<G> {
        let mut inner = self
            .inner
            .borrow_mut()
            .take()
            .expect("TypedParser::parse called while a session is still active");
        // SAFETY: inner.raw is valid (owned via ParserInner); source is
        // copied into source_buf which will be owned by the session.
        unsafe { reset_parser(inner.raw.as_ptr(), &mut inner.source_buf, source) };
        TypedParseSession {
            grammar: self.grammar,
            inner: Some(inner),
            slot: Rc::clone(&self.inner),
            _marker: PhantomData,
        }
    }

    /// Bind source text and return a
    /// [`TypedIncrementalParseSession`](crate::typed::TypedIncrementalParseSession)
    /// for token-by-token feeding.
    ///
    /// # Panics
    ///
    /// Panics if a session from a previous `parse()` or `incremental_parse()`
    /// call is still alive.
    pub fn incremental_parse(&self, source: &str) -> TypedIncrementalParseSession<G> {
        let mut inner = self
            .inner
            .borrow_mut()
            .take()
            .expect("TypedParser::incremental_parse called while a session is still active");
        // SAFETY: inner.raw is valid (owned via ParserInner); source is
        // copied into source_buf.
        unsafe { reset_parser(inner.raw.as_ptr(), &mut inner.source_buf, source) };
        let c_source_ptr =
            NonNull::new(inner.source_buf.as_mut_ptr()).expect("source_buf is non-empty");
        TypedIncrementalParseSession::new(c_source_ptr, self.grammar, inner, Rc::clone(&self.inner))
    }
}

impl TypedParser<AnyGrammar> {
    /// Create a type-erased parser from a [`AnyGrammar`].
    #[allow(dead_code)]
    pub(crate) fn from_raw_grammar(grammar: AnyGrammar) -> Self {
        Self::new(grammar)
    }
}

/// An active session over typed statements from a [`TypedParser`].
///
/// On a parse error the session returns `Some(Err(TypedParseError))` for the
/// failing statement, then continues parsing subsequent statements (Lemon's
/// built-in error recovery synchronises on `;`). Call [`next`](Self::next)
/// again to retrieve the next valid statement.
///
pub struct TypedParseSession<G: TypedGrammar> {
    grammar: AnyGrammar,
    /// Checked-out parser state. Returned to `slot` on drop.
    inner: Option<ParserInner>,
    /// Slot to return `inner` to when this session is dropped.
    slot: Rc<RefCell<Option<ParserInner>>>,
    _marker: PhantomData<G>,
}

impl<G: TypedGrammar> Drop for TypedParseSession<G> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            *self.slot.borrow_mut() = Some(inner);
        }
    }
}

impl<G: TypedGrammar> TypedParseSession<G> {
    /// Parse the next SQL statement.
    ///
    /// Returns:
    /// - `Some(Ok(_))` — successfully parsed statement root.
    /// - `Some(Err(_))` — syntax error; call again to continue
    ///   with subsequent statements (Lemon recovers on `;`). Use
    ///   [`TypedParseError::kind`] to distinguish recovered vs fatal.
    /// - `None` — all input has been consumed.
    ///
    /// # Panics
    ///
    /// Panics if called after the session has been dropped or its inner state
    /// has been reclaimed. This cannot happen in normal use.
    pub fn next(&mut self) -> Option<Result<TypedParsedStatement<'_, G>, TypedParseError<'_, G>>> {
        // SAFETY: raw is valid and exclusively borrowed via &mut self.
        let rc = unsafe {
            self.inner
                .as_mut()
                .expect("inner is Some while session is not finished")
                .raw
                .as_mut()
                .next()
        };

        if rc == ffi::PARSE_DONE {
            return None;
        }

        let inner = self
            .inner
            .as_ref()
            .expect("inner is Some while session is not finished");
        let source_len = inner.source_buf.len().saturating_sub(1);
        // SAFETY: source_buf was populated from valid UTF-8 (&str) in
        // reset_parser. The first source_len bytes are the original source.
        let source = unsafe { std::str::from_utf8_unchecked(&inner.source_buf[..source_len]) };
        // SAFETY: inner.raw is valid (owned via ParserInner, not yet destroyed).
        let result = unsafe { TypedParsedStatement::new(inner.raw.as_ptr(), source, self.grammar) };
        if rc == ffi::PARSE_OK {
            Some(Ok(result))
        } else {
            // RECOVERED or ERROR
            Some(Err(TypedParseError(result)))
        }
    }

    /// The source text bound to this session.
    ///
    /// # Panics
    ///
    /// Panics if called after the session has been dropped or its inner state
    /// has been reclaimed. This cannot happen in normal use.
    pub fn source(&self) -> &str {
        let inner = self
            .inner
            .as_ref()
            .expect("inner is Some while session is not finished");
        let source_len = inner.source_buf.len().saturating_sub(1);
        // SAFETY: source_buf was populated from valid UTF-8 (&str) in
        // reset_parser.
        unsafe { std::str::from_utf8_unchecked(&inner.source_buf[..source_len]) }
    }

    /// Get an [`AnyParsedStatement`] view of this session's arena state.
    ///
    /// Allows reading node data and source text after all statements have been
    /// consumed via [`next`](Self::next). The returned
    /// result borrows from `&self` and is valid as long as this session is alive.
    ///
    /// # Panics
    /// Panics if the inner parser has already been released.
    pub fn arena_result(&self) -> AnyParsedStatement<'_> {
        let inner = self
            .inner
            .as_ref()
            .expect("inner is Some while session is alive");
        let source_len = inner.source_buf.len().saturating_sub(1);
        // SAFETY: source_buf was populated from valid UTF-8 (&str) in
        // reset_parser; inner.raw is valid (owned via ParserInner).
        let source = unsafe { std::str::from_utf8_unchecked(&inner.source_buf[..source_len]) };
        // SAFETY: inner.raw is valid for 'self; source is valid UTF-8 for 'self.
        unsafe { AnyParsedStatement::new(inner.raw.as_ptr(), source, self.grammar) }
    }
}

/// A type-erased parser. Yields [`AnyParseSession`]s with raw node types,
/// suitable for use across multiple dialects.
pub type AnyParser = TypedParser<AnyGrammar>;

/// An active session over raw statements from an [`AnyParser`].
pub type AnyParseSession = TypedParseSession<AnyGrammar>;

/// The result of a successfully parsed SQL statement from a [`TypedParseSession`].
///
/// Provides typed access to the statement root, per-statement token/comment
/// data, and semantic flags.
#[derive(Clone, Copy)]
pub struct TypedParsedStatement<'a, G: TypedGrammar> {
    pub(crate) raw: NonNull<CParser>,
    pub(crate) source: &'a str,
    pub(crate) grammar: AnyGrammar,
    _marker: PhantomData<G>,
}

impl<'a, G: TypedGrammar> TypedParsedStatement<'a, G> {
    /// Construct from raw parts.
    ///
    /// # Safety
    /// `raw` must be a valid, non-null parser pointer that remains valid for `'a`.
    pub(crate) unsafe fn new(raw: *mut CParser, source: &'a str, grammar: AnyGrammar) -> Self {
        TypedParsedStatement {
            // SAFETY: caller guarantees raw is non-null (documented in Safety section above).
            raw: unsafe { NonNull::new_unchecked(raw) },
            source,
            grammar,
            _marker: PhantomData,
        }
    }

    /// Erase the grammar type parameter.
    ///
    /// Required when passing this result to [`GrammarNodeType::from_result`],
    /// which expects a type-erased `stmt_result`.
    pub fn erase(self) -> AnyParsedStatement<'a> {
        TypedParsedStatement {
            raw: self.raw,
            source: self.source,
            grammar: self.grammar,
            _marker: PhantomData,
        }
    }

    /// The typed root node for this statement, or `None` if unavailable.
    pub fn root(&self) -> Option<G::Node<'a>> {
        // SAFETY: self.raw is a valid, non-null parser pointer for lifetime 'a.
        let id = AnyNodeId(unsafe { self.raw.as_ref().result_root() });
        if id.is_null() {
            return None;
        }
        G::Node::from_result(self.erase(), id)
    }

    /// The source text bound to this result.
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// Per-statement token positions. Requires `collect_tokens: true`.
    ///
    /// Skips tokens whose type is not recognised by grammar `G`.
    pub fn tokens(&self) -> impl Iterator<Item = TypedParserToken<'a, G>> {
        let source = self.source;
        // SAFETY: self.raw is valid for 'a; the returned slice lives for 'a.
        let raw: &'a [ffi::CParserToken] = unsafe { self.raw.as_ref().result_tokens() };
        raw.iter().filter_map(move |t| {
            let token_type = G::Token::from_token_type(AnyTokenType(t.type_))?;
            let text = &source[t.offset as usize..(t.offset + t.length) as usize];
            Some(TypedParserToken {
                text,
                token_type,
                flags: ParserTokenFlags::from_raw(t.flags),
            })
        })
    }

    /// Per-statement comments. Requires `collect_tokens: true`.
    pub fn comments(&self) -> impl Iterator<Item = Comment<'a>> {
        let source = self.source;
        // SAFETY: self.raw is valid for 'a; the returned slice lives for 'a.
        let raw: &'a [ffi::CComment] = unsafe { self.raw.as_ref().result_comments() };
        raw.iter().map(move |c| {
            let text = &source[c.offset as usize..(c.offset + c.length) as usize];
            let kind = match c.kind {
                ffi::CCommentKind::LineComment => CommentKind::Line,
                ffi::CCommentKind::BlockComment => CommentKind::Block,
            };
            Comment { text, kind }
        })
    }

    // ── Result accessors (mirror syntaqlite_result_*) ──────────────────────

    /// Human-readable error message, or `None`.
    pub(crate) fn error_msg(&self) -> Option<&str> {
        // SAFETY: self.raw is a valid, non-null parser pointer for lifetime 'a.
        unsafe {
            let ptr = self.raw.as_ref().result_error_msg();
            if ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(ptr).to_str().unwrap_or("parse error"))
            }
        }
    }

    /// Byte offset of the error token, or `None` if unknown.
    pub(crate) fn error_offset(&self) -> Option<usize> {
        // SAFETY: self.raw is a valid, non-null parser pointer for lifetime 'a.
        let v = unsafe { self.raw.as_ref().result_error_offset() };
        if v == 0xFFFF_FFFF {
            None
        } else {
            Some(v as usize)
        }
    }

    /// Byte length of the error token, or `None` if unknown.
    pub(crate) fn error_length(&self) -> Option<usize> {
        // SAFETY: self.raw is a valid, non-null parser pointer for lifetime 'a.
        let v = unsafe { self.raw.as_ref().result_error_length() };
        if v == 0 { None } else { Some(v as usize) }
    }

    /// Error classification for the current result.
    pub(crate) fn error_kind(&self) -> ParseErrorKind {
        // SAFETY: self.raw is a valid, non-null parser pointer for lifetime 'a.
        let v = unsafe { self.raw.as_ref().result_error_kind() };
        ParseErrorKind::from_raw(v)
    }

    // ── Arena access ───────────────────────────────────────────────────────

    /// Resolve a `AnyNodeId` to a typed reference, validating the tag.
    /// Returns `None` if null, invalid, or tag mismatch.
    pub(crate) fn resolve_as<T: ArenaNode>(&self, id: AnyNodeId) -> Option<&'a T> {
        let (ptr, tag) = self.node_ptr(id)?;
        if tag.0 != T::TAG {
            return None;
        }
        // SAFETY: tag matches T::TAG, confirming the arena node has type T.
        // ptr is valid for 'a. T is #[repr(C)] with a u32 tag as its first
        // field, matching the arena layout.
        Some(unsafe { &*ptr.cast::<T>() })
    }

    /// Resolve a `AnyNodeId` as a [`RawNodeList`] (for list nodes).
    /// Returns `None` if null or invalid.
    pub(crate) fn resolve_list(&self, id: AnyNodeId) -> Option<&'a RawNodeList> {
        let (ptr, _) = self.node_ptr(id)?;
        // SAFETY: ptr is valid for 'a. List nodes have RawNodeList layout
        // (tag, count, children[count]). The caller is responsible for
        // ensuring the id refers to a list node (enforced by codegen).
        // The arena guarantees alignment of all allocated nodes.
        #[allow(clippy::cast_ptr_alignment)]
        Some(unsafe { &*ptr.cast::<RawNodeList>() })
    }

    /// Get a raw pointer to a node in the arena. Returns `(pointer, tag)`.
    pub(crate) fn node_ptr(&self, id: AnyNodeId) -> Option<(*const u8, AnyNodeTag)> {
        if id.is_null() {
            return None;
        }
        // SAFETY: self.raw is valid for 'a. The returned pointer is
        // null-checked; all arena nodes start with a u32 tag.
        unsafe {
            let ptr = self.raw.as_ref().node(id.0);
            if ptr.is_null() {
                return None;
            }
            let tag = AnyNodeTag(*ptr);
            Some((ptr.cast::<u8>(), tag))
        }
    }

    /// Dump an AST node tree as indented text into `out`.
    pub(crate) fn dump_node(&self, id: AnyNodeId, out: &mut String, indent: usize) {
        unsafe extern "C" {
            fn free(ptr: *mut std::ffi::c_void);
        }
        // SAFETY: raw is valid; dump_node returns a malloc'd NUL-terminated
        // string (or null). We free it after copying.
        #[allow(clippy::cast_possible_truncation, clippy::cast_ptr_alignment)]
        unsafe {
            let ptr = self.raw.as_ref().dump_node(id.0, indent as u32);
            if !ptr.is_null() {
                out.push_str(&CStr::from_ptr(ptr).to_string_lossy());
                free(ptr.cast::<std::ffi::c_void>());
            }
        }
    }
}

/// A type-erased statement result. The grammar type parameter is fixed to [`AnyGrammar`].
pub type AnyParsedStatement<'a> = TypedParsedStatement<'a, AnyGrammar>;

// ── AnyGrammar-specific statement result APIs ────────────────────────────────

impl<'a> TypedParsedStatement<'a, AnyGrammar> {
    /// Root node ID for the current statement (`AnyNodeId::NULL` if none).
    ///
    /// This is the untyped counterpart to [`TypedParsedStatement::root`], which
    /// returns the dialect's typed node. Use when working in a grammar-agnostic
    /// context, e.g. when passing to [`extract_fields`](Self::extract_fields).
    pub fn root_id(&self) -> AnyNodeId {
        // SAFETY: self.raw is a valid, non-null parser pointer for lifetime 'a.
        AnyNodeId(unsafe { self.raw.as_ref().result_root() })
    }

    /// Macro invocation regions recorded during parsing.
    ///
    /// Each region describes a macro call site's byte range in the original
    /// source. Empty if no macro expansions occurred.
    pub fn macro_regions(&self) -> impl Iterator<Item = MacroRegion> + use<'_> {
        // SAFETY: self.raw is valid for 'a; the slice lives for the parser lifetime.
        let raw: &[ffi::CMacroRegion] = unsafe { self.raw.as_ref().result_macros() };
        raw.iter().map(|r| MacroRegion {
            call_offset: r.call_offset,
            call_length: r.call_length,
        })
    }

    /// Extract the tag and fields from the node at `id`.
    ///
    /// Returns `Some((tag, fields))` where `tag` is the node type ordinal and
    /// `fields` is an indexable collection of [`crate::ast::FieldValue`]s, or `None` if
    /// `id` is null or invalid.
    pub fn extract_fields(
        &self,
        id: AnyNodeId,
    ) -> Option<(AnyNodeTag, crate::ast::NodeFields<'a>)> {
        let (ptr, tag) = self.node_ptr(id)?;
        let mut fields = crate::ast::NodeFields::new();
        for meta in self.grammar.field_meta(tag) {
            // SAFETY: ptr is a valid arena node pointer valid for 'a;
            // meta describes a field within that node's struct layout.
            let val = unsafe { extract_field_value(ptr, &meta, self.source) };
            fields.push(val);
        }
        Some((tag, fields))
    }

    /// Get the child node IDs of a list node.
    ///
    /// Returns `Some(children)` if `id` refers to a list node, `None` if
    /// `id` is null, invalid, or refers to a non-list node.
    pub fn list_children(&self, id: AnyNodeId) -> Option<&'a [AnyNodeId]> {
        let (_, tag) = self.node_ptr(id)?;
        if !self.grammar.is_list(tag) {
            return None;
        }
        #[allow(clippy::redundant_closure_for_method_calls)]
        self.resolve_list(id).map(|l| l.children())
    }

    /// Iterate the immediate child node IDs of the node at `id`.
    ///
    /// For regular nodes, yields each non-null `NodeId` field.
    /// For fields that point to list nodes, yields the list's non-null children instead.
    pub fn child_node_ids(&self, id: AnyNodeId) -> impl Iterator<Item = AnyNodeId> + '_ {
        let mut out = Vec::new();
        if let Some((_, fields)) = self.extract_fields(id) {
            for i in 0..fields.len() {
                if let crate::ast::FieldValue::NodeId(child_id) = fields[i] {
                    if child_id.is_null() {
                        continue;
                    }
                    if let Some(children) = self.list_children(child_id) {
                        out.extend(children.iter().copied().filter(|id| !id.is_null()));
                    } else {
                        out.push(child_id);
                    }
                }
            }
        }
        out.into_iter()
    }
}

/// Extract a single [`crate::ast::FieldValue`] from a raw arena node pointer.
///
/// # Safety
/// `ptr` must point to a valid arena node struct whose field at `meta.offset()`
/// has the type indicated by `meta.kind()`, and must be valid for lifetime `'a`.
#[allow(clippy::cast_ptr_alignment)]
unsafe fn extract_field_value<'a>(
    ptr: *const u8,
    meta: &crate::grammar::FieldMeta<'_>,
    source: &'a str,
) -> crate::ast::FieldValue<'a> {
    use crate::ast::{FieldValue, SourceSpan};
    use crate::grammar::FieldKind;
    // SAFETY: covered by function-level contract; ptr and meta are consistent.
    unsafe {
        let field_ptr = ptr.add(meta.offset() as usize);
        match meta.kind() {
            FieldKind::NodeId => FieldValue::NodeId(AnyNodeId(*(field_ptr.cast::<u32>()))),
            FieldKind::Span => {
                let span = &*(field_ptr.cast::<SourceSpan>());
                if span.length == 0 {
                    FieldValue::Span("")
                } else {
                    FieldValue::Span(span.as_str(source))
                }
            }
            FieldKind::Bool => FieldValue::Bool(*(field_ptr.cast::<u32>()) != 0),
            FieldKind::Flags => FieldValue::Flags(*field_ptr),
            FieldKind::Enum => FieldValue::Enum(*(field_ptr.cast::<u32>())),
        }
    }
}

/// A parse error with human-readable message, optional source location, and
/// optionally a partial recovery tree. Parameterised by dialect grammar `G`.
///
/// Obtain via [`ParseSession::next`] or [`TypedParseSession::next`].
pub struct TypedParseError<'a, G: TypedGrammar>(TypedParsedStatement<'a, G>);

impl<'a, G: TypedGrammar> TypedParseError<'a, G> {
    pub(crate) fn new(result: TypedParsedStatement<'a, G>) -> Self {
        TypedParseError(result)
    }

    /// Returns whether the error is recovered or fatal.
    pub fn kind(&self) -> ParseErrorKind {
        self.0.error_kind()
    }

    /// True if this error was recovered and yielded a partial tree.
    pub fn is_recovered(&self) -> bool {
        self.kind() == ParseErrorKind::Recovered
    }

    /// True if this error is fatal (unrecoverable).
    pub fn is_fatal(&self) -> bool {
        self.kind() == ParseErrorKind::Fatal
    }

    /// Returns the human-readable error message.
    pub fn message(&self) -> &str {
        self.0.error_msg().unwrap_or("parse error")
    }
    /// Returns the byte offset of the error token, or `None` if unknown.
    pub fn offset(&self) -> Option<usize> {
        self.0.error_offset()
    }
    /// Returns the byte length of the error token, or `None` if unknown.
    pub fn length(&self) -> Option<usize> {
        self.0.error_length()
    }
    /// The partial recovery tree, if error recovery produced one.
    pub fn root(&self) -> Option<G::Node<'a>> {
        self.0.root()
    }
}

impl<G: TypedGrammar> std::fmt::Debug for TypedParseError<'_, G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedParseError")
            .field("kind", &self.kind())
            .field("message", &self.message())
            .field("offset", &self.offset())
            .field("length", &self.length())
            .finish()
    }
}

impl<G: TypedGrammar> std::fmt::Display for TypedParseError<'_, G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl<G: TypedGrammar> std::error::Error for TypedParseError<'_, G> {}

/// A type-erased parse error. Yields raw node types, suitable for use across
/// multiple dialects.
pub type AnyParseError<'a> = TypedParseError<'a, AnyGrammar>;

// ── Public token/comment types ───────────────────────────────────────────────

/// The kind of a SQL comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentKind {
    /// A line comment starting with `--`.
    Line,
    /// A block comment delimited by `/* ... */`.
    Block,
}

/// A comment found during parsing.
///
/// Returned by [`TypedParsedStatement::comments`]. Requires
/// `collect_tokens: true` in [`ParserConfig`].
#[derive(Debug, Clone, Copy)]
pub struct Comment<'a> {
    /// The full comment text, including delimiters.
    pub text: &'a str,
    /// Whether this is a line (`--`) or block (`/* */`) comment.
    pub kind: CommentKind,
}

pub use crate::grammar::ParserTokenFlags;

/// A recorded token position from a parsed statement, typed by grammar `G`.
///
/// Returned by [`TypedParsedStatement::tokens`]. Requires
/// `collect_tokens: true` in [`ParserConfig`].
#[derive(Debug, Clone, Copy)]
pub struct TypedParserToken<'a, G: TypedGrammar> {
    text: &'a str,
    token_type: G::Token,
    flags: ParserTokenFlags,
}

impl<'a, G: TypedGrammar> TypedParserToken<'a, G> {
    /// The source text slice covered by this token.
    pub fn text(&self) -> &'a str {
        self.text
    }

    /// Dialect-typed token variant.
    pub fn token_type(&self) -> G::Token {
        self.token_type
    }

    /// Usage flags set by the parser during disambiguation.
    pub fn flags(&self) -> ParserTokenFlags {
        self.flags
    }
}

/// A type-erased token position, not tied to any specific dialect.
///
/// This is [`TypedParserToken`] with the grammar parameter fixed to [`AnyGrammar`].
pub type AnyParserToken<'a> = TypedParserToken<'a, AnyGrammar>;

/// A recorded macro invocation region in the source text.
///
/// Describes the byte range of a macro call site in the original source.
/// Returned by [`AnyParsedStatement::macro_regions`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacroRegion {
    /// Byte offset of the macro call in the original source.
    pub call_offset: u32,
    /// Byte length of the entire macro call.
    pub call_length: u32,
}

/// Semantic completion context at the current parser state.
///
/// Returned by incremental parse sessions for completion engines.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum CompletionContext {
    /// Could not determine context.
    #[default]
    Unknown = 0,
    /// Parser expects an expression.
    Expression = 1,
    /// Parser expects a table reference.
    TableRef = 2,
}

impl CompletionContext {
    /// Convert from a raw completion-context code.
    pub fn from_raw(v: u32) -> Self {
        match v {
            1 => Self::Expression,
            2 => Self::TableRef,
            _ => Self::Unknown,
        }
    }

    /// Return the raw completion-context code.
    pub fn raw(self) -> u32 {
        self as u32
    }
}

impl From<CompletionContext> for u32 {
    fn from(v: CompletionContext) -> u32 {
        v.raw()
    }
}

// ── Crate-internal ───────────────────────────────────────────────────────────

/// Holds the C parser handle and mutable state. Checked out by sessions at
/// runtime and returned on [`Drop`].
pub(crate) struct ParserInner {
    pub(crate) raw: NonNull<CParser>,
    pub(crate) source_buf: Vec<u8>,
}

impl Drop for ParserInner {
    fn drop(&mut self) {
        // SAFETY: self.raw was allocated by CParser::create and has not been
        // freed (Drop runs exactly once).
        unsafe { CParser::destroy(self.raw.as_ptr()) }
    }
}

// ── Incremental parse sessions ────────────────────────────────────────────────

use std::ops::Range;

/// A type-safe incremental parse session for a specific dialect `G`.
///
/// Feed tokens one at a time via `feed_token` and signal
/// end of input with `finish`.
///
/// For the `SQLite` dialect use [`IncrementalParseSession`]. For grammar-agnostic
/// use with raw grammars use [`AnyIncrementalParseSession`].
///
/// Obtained via `TypedParser::incremental_parse`.
pub struct TypedIncrementalParseSession<G: TypedGrammar> {
    /// Base pointer into the internal source buffer. `feed_token` uses this
    /// to compute the C-side token pointer from byte-offset spans.
    #[allow(dead_code)]
    c_source_ptr: NonNull<u8>,
    #[allow(dead_code)]
    grammar: AnyGrammar,
    /// Checked-out parser state. Returned to `slot` on drop.
    inner: Option<ParserInner>,
    /// Slot to return `inner` to when this session is dropped.
    slot: Rc<RefCell<Option<ParserInner>>>,
    finished: bool,
    _marker: PhantomData<G>,
}

impl<G: TypedGrammar> Drop for TypedIncrementalParseSession<G> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            *self.slot.borrow_mut() = Some(inner);
        }
    }
}

#[allow(dead_code)]
impl<G: TypedGrammar> TypedIncrementalParseSession<G> {
    pub(crate) fn new(
        c_source_ptr: NonNull<u8>,
        grammar: AnyGrammar,
        inner: ParserInner,
        slot: Rc<RefCell<Option<ParserInner>>>,
    ) -> Self {
        TypedIncrementalParseSession {
            c_source_ptr,
            grammar,
            inner: Some(inner),
            slot,
            finished: false,
            _marker: PhantomData,
        }
    }

    fn assert_not_finished(&self) {
        assert!(
            !self.finished,
            "TypedIncrementalParseSession used after finish()"
        );
    }

    fn raw_ptr(&self) -> *mut CParser {
        self.inner
            .as_ref()
            .expect("inner taken after finish()")
            .raw
            .as_ptr()
    }

    fn typed_stmt_result(&self) -> TypedParsedStatement<'_, G> {
        let inner = self.inner.as_ref().expect("inner taken after finish()");
        let source_len = inner.source_buf.len().saturating_sub(1);
        // SAFETY: source_buf was populated from valid UTF-8 (&str) in
        // reset_parser. The first source_len bytes are the original source.
        let source = unsafe { std::str::from_utf8_unchecked(&inner.source_buf[..source_len]) };
        // SAFETY: inner.raw is valid (owned via ParserInner, not yet destroyed).
        unsafe { TypedParsedStatement::new(inner.raw.as_ptr(), source, self.grammar) }
    }

    fn result_from_rc(
        &self,
        rc: i32,
    ) -> Option<Result<TypedParsedStatement<'_, G>, TypedParseError<'_, G>>> {
        if rc == 0 {
            return None;
        }
        let result = self.typed_stmt_result();
        if rc == 1 {
            Some(Ok(result))
        } else {
            Some(Err(TypedParseError::new(result)))
        }
    }

    /// Feed a single token to the parser.
    ///
    /// `TK_SPACE` is silently skipped. `TK_COMMENT` is recorded as a comment
    /// (when `collect_tokens` is enabled) but not fed to the parser.
    ///
    /// Returns:
    /// - `None` — keep going, statement not yet complete.
    /// - `Some(Ok(result))` — statement parsed cleanly; use
    ///   [`TypedParsedStatement::root`] to access the typed AST.
    /// - `Some(Err(err))` — parse error; `err.root()` may contain a partial
    ///   recovery tree.
    ///
    /// `span` is a byte range into the source text bound by this session.
    /// `token_type` is the grammar's typed token enum.
    pub fn feed_token(
        &mut self,
        token_type: G::Token,
        span: Range<usize>,
    ) -> Option<Result<TypedParsedStatement<'_, G>, TypedParseError<'_, G>>> {
        self.assert_not_finished();
        // SAFETY: c_source_ptr is valid for the source length; raw is valid.
        let rc = unsafe {
            let c_text = self.c_source_ptr.as_ptr().add(span.start);
            let raw_token_type: u32 = token_type.into();
            #[allow(clippy::cast_possible_truncation)]
            (*self.raw_ptr()).feed_token(raw_token_type, c_text as *const _, span.len() as u32)
        };
        self.result_from_rc(rc)
    }

    /// Signal end of input.
    ///
    /// Synthesizes a SEMI if the last token wasn't one, then sends EOF to the
    /// parser. Returns:
    /// - `None` — nothing was pending (empty input or bare semicolons only).
    /// - `Some(Ok(result))` — final statement parsed cleanly.
    /// - `Some(Err(err))` — parse error; `err.root()` may contain a partial
    ///   recovery tree.
    ///
    /// No further methods may be called after `finish()`.
    pub fn finish(
        &mut self,
    ) -> Option<Result<TypedParsedStatement<'_, G>, TypedParseError<'_, G>>> {
        self.assert_not_finished();
        self.finished = true;
        // SAFETY: raw is valid.
        let rc = unsafe { (*self.raw_ptr()).finish() };
        self.result_from_rc(rc)
    }

    /// Return the token types that are valid lookaheads at the current parser state.
    ///
    /// Useful for completion engines after feeding tokens up to the session.
    /// Unknown token ordinals (not representable as `G::Token`) are silently dropped.
    pub fn expected_tokens(&self) -> impl Iterator<Item = <G as TypedGrammar>::Token> {
        self.assert_not_finished();
        let raw = self.raw_ptr();
        let mut stack_buf = [0u32; 256];
        // SAFETY: raw is valid and exclusively borrowed via &self; stack_buf is
        // a valid output buffer.
        #[allow(clippy::cast_possible_truncation)]
        let total =
            unsafe { (*raw).expected_tokens(stack_buf.as_mut_ptr(), stack_buf.len() as u32) };
        let raw_tokens: Vec<u32> = if total == 0 {
            Vec::new()
        } else {
            let count = total as usize;
            if count <= stack_buf.len() {
                stack_buf[..count].to_vec()
            } else {
                let mut heap_buf = vec![0u32; count];
                // SAFETY: raw is valid; heap_buf is sized to hold `total` entries.
                let written = unsafe { (*raw).expected_tokens(heap_buf.as_mut_ptr(), total) };
                let len = written.clamp(0, total) as usize;
                heap_buf.truncate(len);
                heap_buf
            }
        };
        raw_tokens
            .into_iter()
            .map(AnyTokenType)
            .filter_map(<G as TypedGrammar>::Token::from_token_type)
    }

    /// Return the semantic completion context at the current parser state.
    pub fn completion_context(&self) -> CompletionContext {
        self.assert_not_finished();
        // SAFETY: raw is valid and exclusively borrowed via &self.
        unsafe { (*self.raw_ptr()).completion_context() }
    }

    /// Return the number of nodes currently in the parser arena.
    pub fn node_count(&self) -> u32 {
        // SAFETY: raw is valid and exclusively borrowed via &self.
        unsafe { (*self.raw_ptr()).node_count() }
    }

    /// Mark subsequent fed tokens as being inside a macro expansion.
    ///
    /// `span` describes the macro call's byte range in the original source.
    /// Calls may nest (for nested macro expansions).
    pub fn begin_macro(&mut self, span: Range<usize>) {
        self.assert_not_finished();
        let call_offset = u32::try_from(span.start).expect("macro span start exceeds u32");
        let call_length = u32::try_from(span.len()).expect("macro span length exceeds u32");
        // SAFETY: raw is valid and exclusively borrowed via &mut self.
        unsafe { (*self.raw_ptr()).begin_macro(call_offset, call_length) }
    }

    /// End the innermost macro expansion region.
    pub fn end_macro(&mut self) {
        self.assert_not_finished();
        // SAFETY: raw is valid and exclusively borrowed via &mut self.
        unsafe { (*self.raw_ptr()).end_macro() }
    }

    pub(crate) fn stmt_result(&self) -> AnyParsedStatement<'_> {
        self.typed_stmt_result().erase()
    }

    pub(crate) fn node_ref(&self, id: AnyNodeId) -> AnyNode<'_> {
        AnyNode {
            id,
            stmt_result: self.stmt_result(),
        }
    }

    pub(crate) fn comments(&self) -> &[ffi::CComment] {
        // SAFETY: raw is valid (owned via ParserInner, valid for &self).
        unsafe { (*self.raw_ptr()).result_comments() }
    }

    pub(crate) fn tokens(&self) -> &[ffi::CParserToken] {
        // SAFETY: raw is valid (owned via ParserInner, valid for &self).
        unsafe { (*self.raw_ptr()).result_tokens() }
    }

    pub(crate) fn macro_regions(&self) -> &[ffi::CMacroRegion] {
        // SAFETY: raw is valid (owned via ParserInner, valid for &self).
        unsafe { (*self.raw_ptr()).result_macros() }
    }
}

/// A type-erased incremental parse session. Yields type-erased statement
/// results with raw node types, suitable for use across multiple dialects.
pub type AnyIncrementalParseSession = TypedIncrementalParseSession<AnyGrammar>;

/// An incremental parse session for the `SQLite` dialect. Produced by
/// [`Parser::incremental_parse`].
///
/// Feed tokens one at a time via [`feed_token`](Self::feed_token) and signal
/// end of input with [`finish`](Self::finish).
///
/// On drop, the checked-out parser state is returned to the parent [`Parser`].
#[cfg(feature = "sqlite")]
pub struct IncrementalParseSession(TypedIncrementalParseSession<crate::sqlite::grammar::Grammar>);

#[cfg(feature = "sqlite")]
#[allow(dead_code)]
impl IncrementalParseSession {
    /// Feed a single token to the parser.
    ///
    /// Returns:
    /// - `None` — keep going, statement not yet complete.
    /// - `Some(Ok(result))` — statement parsed cleanly.
    /// - `Some(Err(e))` — parse error; `e.root()` may contain a partial
    ///   recovery tree.
    /// `span` is a byte range into the source text bound by this session.
    pub fn feed_token(
        &mut self,
        token_type: crate::sqlite::tokens::TokenType,
        span: Range<usize>,
    ) -> Option<Result<ParsedStatement<'_>, ParseError<'_>>> {
        Some(match self.0.feed_token(token_type, span)? {
            Ok(result) => Ok(ParsedStatement(result)),
            Err(err) => Err(ParseError(err)),
        })
    }

    /// Signal end of input.
    ///
    /// Returns:
    /// - `None` — nothing was pending.
    /// - `Some(Ok(result))` — final statement parsed cleanly.
    /// - `Some(Err(e))` — parse error; `e.root()` may contain a partial
    ///   recovery tree.
    ///
    /// No further methods may be called after `finish()`.
    pub fn finish(&mut self) -> Option<Result<ParsedStatement<'_>, ParseError<'_>>> {
        Some(match self.0.finish()? {
            Ok(result) => Ok(ParsedStatement(result)),
            Err(err) => Err(ParseError(err)),
        })
    }

    /// Return the token types that are valid lookaheads at the current parser state.
    pub fn expected_tokens(&self) -> impl Iterator<Item = crate::sqlite::tokens::TokenType> {
        self.0.expected_tokens()
    }

    /// Return the semantic completion context at the current parser state.
    pub fn completion_context(&self) -> CompletionContext {
        self.0.completion_context()
    }

    /// Return the number of nodes currently in the parser arena.
    pub fn node_count(&self) -> u32 {
        self.0.node_count()
    }

    /// Mark subsequent fed tokens as being inside a macro expansion.
    pub fn begin_macro(&mut self, span: Range<usize>) {
        self.0.begin_macro(span);
    }

    /// End the innermost macro expansion region.
    pub fn end_macro(&mut self) {
        self.0.end_macro();
    }

    pub(crate) fn stmt_result(&self) -> AnyParsedStatement<'_> {
        self.0.stmt_result()
    }

    pub(crate) fn node_ref(&self, id: AnyNodeId) -> AnyNode<'_> {
        self.0.node_ref(id)
    }

    pub(crate) fn comments(&self) -> &[ffi::CComment] {
        self.0.comments()
    }

    pub(crate) fn tokens(&self) -> &[ffi::CParserToken] {
        self.0.tokens()
    }

    pub(crate) fn macro_regions(&self) -> &[ffi::CMacroRegion] {
        self.0.macro_regions()
    }
}

#[cfg(feature = "sqlite")]
impl From<TypedIncrementalParseSession<crate::sqlite::grammar::Grammar>>
    for IncrementalParseSession
{
    fn from(inner: TypedIncrementalParseSession<crate::sqlite::grammar::Grammar>) -> Self {
        IncrementalParseSession(inner)
    }
}

/// Copy source into `source_buf` (with null terminator) and reset the C parser.
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
    #[allow(clippy::cast_possible_truncation)]
    unsafe {
        (*raw).reset(c_source_ptr.cast(), source.len() as u32);
    }
}

// ── ffi ───────────────────────────────────────────────────────────────────────

mod ffi {
    use std::ffi::{c_char, c_void};

    /// Opaque C parser type.
    pub(crate) enum CParser {}

    /// Return code: no statement / done.
    pub(crate) const PARSE_DONE: i32 = 0;
    /// Return code: statement parsed cleanly.
    pub(crate) const PARSE_OK: i32 = 1;
    /// Return code: statement parsed with error recovery.
    #[allow(dead_code)]
    pub(crate) const PARSE_RECOVERED: i32 = 2;
    /// Return code: unrecoverable error.
    #[allow(dead_code)]
    pub(crate) const PARSE_ERROR: i32 = -1;

    /// Mirrors C `SyntaqliteMemMethods`.
    #[repr(C)]
    pub(crate) struct CMemMethods {
        pub x_malloc: unsafe extern "C" fn(usize) -> *mut c_void,
        pub x_free: unsafe extern "C" fn(*mut c_void),
    }

    /// The kind of a comment.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    #[allow(dead_code)]
    pub(crate) enum CCommentKind {
        LineComment = 0,
        BlockComment = 1,
    }

    /// Mirrors C `SyntaqliteComment`.
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub(crate) struct CComment {
        pub offset: u32,
        pub length: u32,
        pub kind: CCommentKind,
    }

    #[allow(dead_code)]
    pub(super) const TOKEN_FLAG_AS_ID: u32 = 1;
    #[allow(dead_code)]
    pub(super) const TOKEN_FLAG_AS_FUNCTION: u32 = 2;
    #[allow(dead_code)]
    pub(super) const TOKEN_FLAG_AS_TYPE: u32 = 4;

    /// Mirrors C `SyntaqliteCompletionContext` (`typedef uint32_t`).
    pub(crate) type CCompletionContext = u32;

    /// Mirrors C `SyntaqliteParserTokenFlags` (`typedef uint32_t`).
    pub(crate) type CParserTokenFlags = u32;

    /// Mirrors C `SyntaqliteParserToken`.
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub(crate) struct CParserToken {
        pub offset: u32,
        pub length: u32,
        pub type_: u32,
        pub flags: CParserTokenFlags,
    }

    /// A recorded macro invocation region.
    ///
    /// Mirrors C `SyntaqliteMacroRegion` from `include/syntaqlite/parser.h`.
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub(crate) struct CMacroRegion {
        /// Byte offset of the macro call in the original source.
        pub(crate) call_offset: u32,
        /// Byte length of the entire macro call.
        pub(crate) call_length: u32,
    }

    /// Tag value for error placeholder nodes (tag 0).
    #[allow(dead_code)]
    pub(super) const SYNTAQLITE_ERROR_NODE_TAG: u32 = 0;

    /// Mirrors C `SyntaqliteErrorNode`.
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub(super) struct CErrorNode {
        pub tag: u32,
        pub offset: u32,
        pub length: u32,
    }
    use std::mem::size_of;
    const _: () = assert!(size_of::<CErrorNode>() == 12);

    impl CParser {
        // Lifecycle
        pub(crate) unsafe fn create(
            mem: *const CMemMethods,
            grammar: crate::grammar::ffi::CGrammar,
        ) -> *mut Self {
            // SAFETY: mem may be null (use default allocator); grammar is a
            // valid grammar handle passed by the caller.
            unsafe { syntaqlite_create_parser_with_grammar(mem, grammar) }
        }

        pub(crate) unsafe fn set_trace(&mut self, enable: u32) -> i32 {
            // SAFETY: self is a valid, non-null CParser pointer owned by the caller.
            unsafe { syntaqlite_parser_set_trace(self, enable) }
        }

        pub(crate) unsafe fn set_collect_tokens(&mut self, enable: u32) -> i32 {
            // SAFETY: self is a valid, non-null CParser pointer owned by the caller.
            unsafe { syntaqlite_parser_set_collect_tokens(self, enable) }
        }

        pub(crate) unsafe fn reset(&mut self, source: *const c_char, len: u32) {
            // SAFETY: self is a valid, non-null CParser pointer; source is a
            // null-terminated C string of at least `len` bytes.
            unsafe { syntaqlite_parser_reset(self, source, len) }
        }

        pub(crate) unsafe fn next(&mut self) -> i32 {
            // SAFETY: self is a valid, non-null CParser pointer owned by the caller.
            unsafe { syntaqlite_parser_next(self) }
        }

        pub(crate) unsafe fn destroy(this: *mut Self) {
            // SAFETY: this is a valid CParser pointer previously created by
            // `syntaqlite_create_parser_with_grammar` and not yet destroyed.
            unsafe { syntaqlite_parser_destroy(this) }
        }

        // Result accessors (valid after `next()` returns non-DONE)
        pub(crate) unsafe fn result_root(&self) -> u32 {
            // SAFETY: self is a valid, non-null CParser pointer; result
            // accessors are valid after `next()` returns a non-DONE code.
            unsafe { syntaqlite_result_root(std::ptr::from_ref::<Self>(self).cast_mut()) }
        }

        pub(crate) unsafe fn result_error_kind(&self) -> u32 {
            // SAFETY: self is a valid, non-null CParser pointer; result
            // accessors are valid after `next()` returns a non-DONE code.
            unsafe { syntaqlite_result_error_kind(std::ptr::from_ref::<Self>(self).cast_mut()) }
        }

        pub(crate) unsafe fn result_error_msg(&self) -> *const c_char {
            // SAFETY: self is a valid, non-null CParser pointer; result
            // accessors are valid after `next()` returns a non-DONE code.
            unsafe { syntaqlite_result_error_msg(std::ptr::from_ref::<Self>(self).cast_mut()) }
        }

        pub(crate) unsafe fn result_error_offset(&self) -> u32 {
            // SAFETY: self is a valid, non-null CParser pointer; result
            // accessors are valid after `next()` returns a non-DONE code.
            unsafe { syntaqlite_result_error_offset(std::ptr::from_ref::<Self>(self).cast_mut()) }
        }

        pub(crate) unsafe fn result_error_length(&self) -> u32 {
            // SAFETY: self is a valid, non-null CParser pointer; result
            // accessors are valid after `next()` returns a non-DONE code.
            unsafe { syntaqlite_result_error_length(std::ptr::from_ref::<Self>(self).cast_mut()) }
        }

        pub(crate) unsafe fn result_comments(&self) -> &[CComment] {
            let mut count: u32 = 0;
            // SAFETY: self is a valid, non-null CParser pointer; result
            // accessors are valid after `next()` returns a non-DONE code.
            let ptr = unsafe {
                syntaqlite_result_comments(
                    std::ptr::from_ref::<Self>(self).cast_mut(),
                    &raw mut count,
                )
            };
            if count == 0 || ptr.is_null() {
                return &[];
            }
            // SAFETY: ptr is a valid pointer to `count` CComment values owned
            // by the parser arena; the slice is valid for the parser's lifetime.
            unsafe { std::slice::from_raw_parts(ptr, count as usize) }
        }

        pub(crate) unsafe fn result_tokens(&self) -> &[CParserToken] {
            let mut count: u32 = 0;
            // SAFETY: self is a valid, non-null CParser pointer; result
            // accessors are valid after `next()` returns a non-DONE code.
            let ptr = unsafe {
                syntaqlite_result_tokens(
                    std::ptr::from_ref::<Self>(self).cast_mut(),
                    &raw mut count,
                )
            };
            if count == 0 || ptr.is_null() {
                return &[];
            }
            // SAFETY: ptr is a valid pointer to `count` CParserToken values owned
            // by the parser arena; the slice is valid for the parser's lifetime.
            unsafe { std::slice::from_raw_parts(ptr, count as usize) }
        }

        pub(crate) unsafe fn result_macros(&self) -> &[CMacroRegion] {
            let mut count: u32 = 0;
            // SAFETY: self is a valid, non-null CParser pointer; result
            // accessors are valid after `next()` returns a non-DONE code.
            let ptr = unsafe {
                syntaqlite_result_macros(
                    std::ptr::from_ref::<Self>(self).cast_mut(),
                    &raw mut count,
                )
            };
            if count == 0 || ptr.is_null() {
                return &[];
            }
            // SAFETY: ptr is a valid pointer to `count` CMacroRegion values owned
            // by the parser arena; the slice is valid for the parser's lifetime.
            unsafe { std::slice::from_raw_parts(ptr, count as usize) }
        }

        // Arena accessors
        pub(crate) unsafe fn node(&self, node_id: u32) -> *const u32 {
            // SAFETY: self is a valid, non-null CParser pointer; node_id is a
            // raw node ID from the arena (null is handled by the C side).
            unsafe { syntaqlite_parser_node(std::ptr::from_ref::<Self>(self).cast_mut(), node_id) }
        }

        pub(crate) unsafe fn node_count(&self) -> u32 {
            // SAFETY: self is a valid, non-null CParser pointer owned by the caller.
            unsafe { syntaqlite_parser_node_count(std::ptr::from_ref::<Self>(self).cast_mut()) }
        }

        // AST dump
        pub(crate) unsafe fn dump_node(&self, node_id: u32, indent: u32) -> *mut c_char {
            // SAFETY: self is a valid, non-null CParser pointer; node_id is a
            // raw node ID from the arena. Returns a malloc'd string or null.
            unsafe {
                syntaqlite_dump_node(std::ptr::from_ref::<Self>(self).cast_mut(), node_id, indent)
            }
        }

        // Incremental (token-feeding) API
        pub(crate) unsafe fn feed_token(
            &mut self,
            token_type: u32,
            text: *const c_char,
            len: u32,
        ) -> i32 {
            // SAFETY: self is a valid, non-null CParser pointer; text is a
            // valid pointer to at least `len` bytes of token text.
            unsafe { syntaqlite_parser_feed_token(self, token_type, text, len) }
        }

        pub(crate) unsafe fn finish(&mut self) -> i32 {
            // SAFETY: self is a valid, non-null CParser pointer owned by the caller.
            unsafe { syntaqlite_parser_finish(self) }
        }

        pub(crate) unsafe fn expected_tokens(&self, out_tokens: *mut u32, out_cap: u32) -> u32 {
            // SAFETY: self is a valid, non-null CParser pointer; out_tokens
            // is a valid pointer to at least `out_cap` u32 values.
            unsafe {
                syntaqlite_parser_expected_tokens(
                    std::ptr::from_ref::<Self>(self).cast_mut(),
                    out_tokens,
                    out_cap,
                )
            }
        }

        pub(crate) unsafe fn completion_context(&self) -> super::CompletionContext {
            // SAFETY: self is a valid, non-null CParser pointer owned by the caller.
            unsafe {
                super::CompletionContext::from_raw(syntaqlite_parser_completion_context(
                    std::ptr::from_ref::<Self>(self).cast_mut(),
                ))
            }
        }

        pub(crate) unsafe fn begin_macro(&mut self, call_offset: u32, call_length: u32) {
            // SAFETY: self is a valid, non-null CParser pointer owned by the caller.
            unsafe { syntaqlite_parser_begin_macro(self, call_offset, call_length) }
        }

        pub(crate) unsafe fn end_macro(&mut self) {
            // SAFETY: self is a valid, non-null CParser pointer owned by the caller.
            unsafe { syntaqlite_parser_end_macro(self) }
        }
    }

    unsafe extern "C" {
        // Parser lifecycle
        fn syntaqlite_create_parser_with_grammar(
            mem: *const CMemMethods,
            grammar: crate::grammar::ffi::CGrammar,
        ) -> *mut CParser;
        fn syntaqlite_parser_reset(p: *mut CParser, source: *const c_char, len: u32);
        fn syntaqlite_parser_next(p: *mut CParser) -> i32;
        fn syntaqlite_parser_destroy(p: *mut CParser);

        // Result accessors
        fn syntaqlite_result_root(p: *mut CParser) -> u32;
        fn syntaqlite_result_error_kind(p: *mut CParser) -> u32;
        fn syntaqlite_result_error_msg(p: *mut CParser) -> *const c_char;
        fn syntaqlite_result_error_offset(p: *mut CParser) -> u32;
        fn syntaqlite_result_error_length(p: *mut CParser) -> u32;
        fn syntaqlite_result_comments(p: *mut CParser, count: *mut u32) -> *const CComment;
        fn syntaqlite_result_tokens(p: *mut CParser, count: *mut u32) -> *const CParserToken;
        fn syntaqlite_result_macros(p: *mut CParser, count: *mut u32) -> *const CMacroRegion;

        // Arena accessors
        fn syntaqlite_parser_node(p: *mut CParser, node_id: u32) -> *const u32;
        fn syntaqlite_parser_node_count(p: *mut CParser) -> u32;

        // Configuration
        fn syntaqlite_parser_set_trace(p: *mut CParser, enable: u32) -> i32;
        fn syntaqlite_parser_set_collect_tokens(p: *mut CParser, enable: u32) -> i32;

        // AST dump
        fn syntaqlite_dump_node(p: *mut CParser, node_id: u32, indent: u32) -> *mut c_char;

        // Incremental (token-feeding) API (from incremental.h)
        fn syntaqlite_parser_feed_token(
            p: *mut CParser,
            token_type: u32,
            text: *const c_char,
            len: u32,
        ) -> i32;
        fn syntaqlite_parser_finish(p: *mut CParser) -> i32;
        fn syntaqlite_parser_expected_tokens(
            p: *mut CParser,
            out_tokens: *mut u32,
            out_cap: u32,
        ) -> u32;
        fn syntaqlite_parser_completion_context(p: *mut CParser) -> CCompletionContext;
        fn syntaqlite_parser_begin_macro(p: *mut CParser, call_offset: u32, call_length: u32);
        fn syntaqlite_parser_end_macro(p: *mut CParser);
    }
}

pub(crate) use ffi::CParser;

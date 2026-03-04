// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::RefCell;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::rc::Rc;

use crate::ast::{AnyDialect, AnyNode, AnyNodeId, ArenaNode, GrammarNodeType, NodeList};
use crate::grammar::{AnyGrammar, TypedGrammar};

// ── Public API ───────────────────────────────────────────────────────────────

/// Configuration for parser construction.
#[derive(Debug, Default, Clone, Copy)]
pub struct ParserConfig {
    /// Enable parser trace output (Lemon debug trace). Default: `false`.
    pub trace: bool,
    /// Collect non-whitespace token positions during parsing. Default: `false`.
    pub collect_tokens: bool,
}

/// A parser for the `SQLite` dialect. Yields [`ParseSession`]s with
/// SQLite-specific node types. For other dialects use [`TypedParser`] directly;
/// for dialect-agnostic use with raw grammars use [`AnyParser`].
///
/// Owns a parser instance. Reusable across inputs via `parse()` and
/// `incremental_parse()`. Uses an interior-mutability checkout pattern so both
/// methods take `&self` rather than `&mut self`.
#[cfg(feature = "sqlite")]
pub struct Parser(TypedParser<crate::sqlite::grammar::SqliteGrammar>);

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
    /// [`IncrementalParseSession`](crate::incremental::IncrementalParseSession)
    /// for token-by-token feeding.
    ///
    /// # Panics
    ///
    /// Panics if a session from a previous `parse()` or `incremental_parse()`
    /// call is still alive.
    pub fn incremental_parse(&self, source: &str) -> crate::incremental::IncrementalParseSession {
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
/// On a parse error the session returns `Some(Err(_))` for the failing
/// statement, then continues parsing subsequent statements (Lemon's built-in
/// error recovery synchronises on `;`). Call `next_statement()` again to
/// retrieve the next valid statement.
///
/// On drop, the checked-out parser state is returned to the parent [`Parser`].
#[cfg(feature = "sqlite")]
pub struct ParseSession(TypedParseSession<crate::sqlite::grammar::SqliteGrammar>);

#[cfg(feature = "sqlite")]
impl ParseSession {
    /// Parse the next SQL statement.
    ///
    /// Returns:
    /// - `Some(Ok(result))` — successfully parsed statement; use
    ///   [`StatementResult::root`] to access the typed AST,
    ///   [`StatementResult::tokens`] / [`StatementResult::comments`] for
    ///   per-statement token data.
    /// - `Some(Err(e))` — syntax error; the error's [`root()`](ParseError::root)
    ///   may contain a partially recovered tree. Call again to continue with
    ///   subsequent statements (Lemon recovers on `;`).
    /// - `None` — all input has been consumed.
    pub fn next_statement(&mut self) -> Option<Result<StatementResult<'_>, ParseError<'_>>> {
        Some(match self.0.next_statement()? {
            Ok(result) => Ok(result),
            Err(err) => Err(ParseError(err)),
        })
    }

    /// The source text bound to this session.
    pub fn source(&self) -> &str {
        self.0.source()
    }

    /// The grammar for this session.
    pub fn grammar(&self) -> AnyGrammar {
        self.0.grammar()
    }

    #[allow(dead_code)]
    pub(crate) fn stmt_result(&self) -> AnyStatementResult<'_> {
        self.0.stmt_result()
    }

    #[allow(dead_code)]
    pub(crate) fn node_ref(&self, id: AnyNodeId) -> AnyNode<'_> {
        self.0.node_ref(id)
    }
}

/// A type-safe parser scoped to a specific dialect `G`.
///
/// Yields [`TypedParseSession<G>`]s. For the common `SQLite` case use
/// [`Parser`]. For dialect-agnostic use with raw grammars use [`AnyParser`].
///
/// Uses an interior-mutability checkout pattern: `parse()` and
/// `incremental_parse()` check out the C parser state at runtime, and the
/// returned session returns it on drop. This allows both methods to take
/// `&self` rather than `&mut self`.
pub struct TypedParser<G: TypedGrammar> {
    inner: Rc<RefCell<Option<ParserInner>>>,
    grammar: AnyGrammar,
    _marker: PhantomData<G>,
}

impl<G: TypedGrammar> TypedParser<G> {
    /// Create a parser bound to the given dialect grammar with default configuration.
    pub(crate) fn new(grammar: G) -> Self {
        Self::with_config(grammar, &ParserConfig::default())
    }

    /// Create a parser bound to the given dialect grammar with custom configuration.
    pub(crate) fn with_config(mut grammar: G, config: &ParserConfig) -> Self {
        let grammar_raw = *grammar.raw();
        // SAFETY: create(NULL, grammar_raw.inner) allocates a new parser with
        // default malloc/free. The C side copies the grammar.
        let mut raw = NonNull::new(unsafe { CParser::create(std::ptr::null(), grammar_raw.inner) })
            .expect("parser allocation failed");

        // SAFETY: raw is freshly created (not sealed), so these calls always return 0.
        unsafe {
            raw.as_mut().set_trace(u32::from(config.trace));
            raw.as_mut()
                .set_collect_tokens(u32::from(config.collect_tokens));
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
    pub(crate) fn parse(&self, source: &str) -> TypedParseSession<G> {
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
    /// [`TypedIncrementalParseSession`](crate::incremental::TypedIncrementalParseSession)
    /// for token-by-token feeding.
    ///
    /// # Panics
    ///
    /// Panics if a session from a previous `parse()` or `incremental_parse()`
    /// call is still alive.
    pub(crate) fn incremental_parse(
        &self,
        source: &str,
    ) -> crate::incremental::TypedIncrementalParseSession<G> {
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
        crate::incremental::TypedIncrementalParseSession::new(
            c_source_ptr,
            self.grammar,
            inner,
            Rc::clone(&self.inner),
        )
    }
}

impl TypedParser<AnyDialect> {
    /// Create a type-erased parser from a [`AnyGrammar`].
    #[allow(dead_code)]
    pub(crate) fn from_raw_grammar(grammar: AnyGrammar) -> Self {
        Self::new(AnyDialect { raw: grammar })
    }
}

/// An active session over typed statements from a [`TypedParser`].
///
/// On a parse error the session returns `Some(Err(_))` for the failing
/// statement, then continues parsing subsequent statements (Lemon's built-in
/// error recovery synchronises on `;`). Call `next_statement()` again to
/// retrieve the next valid statement.
///
/// On drop, the checked-out parser state is returned to the parent
/// [`TypedParser`].
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
    /// - `Some(Ok(result))` — successfully parsed statement root.
    /// - `Some(Err(e))` — syntax error; call again to continue with subsequent
    ///   statements (Lemon recovers on `;`).
    /// - `None` — all input has been consumed.
    pub(crate) fn next_statement(
        &mut self,
    ) -> Option<Result<TypedStatementResult<'_, G>, TypedParseError<'_, G>>> {
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
        let result = unsafe { TypedStatementResult::new(inner.raw.as_ptr(), source, self.grammar) };
        if rc == ffi::PARSE_OK {
            Some(Ok(result))
        } else {
            // RECOVERED or ERROR
            Some(Err(TypedParseError(result)))
        }
    }

    /// Build a [`AnyStatementResult`] for the parser arena, borrowing source text
    /// from the internal buffer.
    ///
    /// Lightweight (no allocation) — packages the raw parser pointer with a
    /// `&str` view of the owned source buffer.
    #[allow(dead_code)]
    pub(crate) fn stmt_result(&self) -> AnyStatementResult<'_> {
        let inner = self
            .inner
            .as_ref()
            .expect("inner is Some while session is not finished");
        let source_len = inner.source_buf.len().saturating_sub(1);
        // SAFETY: source_buf was populated from valid UTF-8 (&str) in
        // reset_parser. The first source_len bytes are the original source.
        let source = unsafe { std::str::from_utf8_unchecked(&inner.source_buf[..source_len]) };
        // SAFETY: inner.raw is valid (owned via ParserInner, not yet destroyed).
        unsafe { TypedStatementResult::new(inner.raw.as_ptr(), source, self.grammar) }
    }

    /// The source text bound to this session.
    pub(crate) fn source(&self) -> &str {
        let inner = self
            .inner
            .as_ref()
            .expect("inner is Some while session is not finished");
        let source_len = inner.source_buf.len().saturating_sub(1);
        // SAFETY: source_buf was populated from valid UTF-8 (&str) in
        // reset_parser.
        unsafe { std::str::from_utf8_unchecked(&inner.source_buf[..source_len]) }
    }

    /// The grammar for this session.
    pub(crate) fn grammar(&self) -> AnyGrammar {
        self.grammar
    }

    /// Wrap a `AnyNodeId` into an [`AnyNode`] using this session's `stmt_result`.
    #[allow(dead_code)]
    pub(crate) fn node_ref(&self, id: AnyNodeId) -> AnyNode<'_> {
        AnyNode {
            id,
            stmt_result: self.stmt_result(),
        }
    }

    #[allow(dead_code)]
    fn raw_ptr(&self) -> *mut CParser {
        self.inner
            .as_ref()
            .expect("inner is Some while session is not finished")
            .raw
            .as_ptr()
    }
}

/// A type-erased parser. Yields [`AnyParseSession`]s with raw node types,
/// suitable for use across multiple dialects.
pub type AnyParser = TypedParser<AnyDialect>;

/// An active session over raw statements from an [`AnyParser`].
pub type AnyParseSession = TypedParseSession<AnyDialect>;

/// The result of a successfully parsed SQL statement from a [`TypedParseSession`].
///
/// Provides typed access to the statement root, per-statement token/comment
/// data, and semantic flags. Valid until the next call to
/// [`TypedParseSession::next_statement`].
///
/// This is the primary result struct; [`AnyStatementResult`] is a type alias
/// for `TypedStatementResult<'a, AnyDialect>`.
#[derive(Clone, Copy)]
pub struct TypedStatementResult<'a, G: TypedGrammar> {
    pub(crate) raw: NonNull<CParser>,
    pub(crate) source: &'a str,
    pub(crate) grammar: AnyGrammar,
    _marker: PhantomData<G>,
}

impl<'a, G: TypedGrammar> TypedStatementResult<'a, G> {
    /// Construct from raw parts.
    ///
    /// # Safety
    /// `raw` must be a valid, non-null parser pointer that remains valid for `'a`.
    pub(crate) unsafe fn new(raw: *mut CParser, source: &'a str, grammar: AnyGrammar) -> Self {
        TypedStatementResult {
            // SAFETY: caller guarantees raw is non-null (documented in Safety section above).
            raw: unsafe { NonNull::new_unchecked(raw) },
            source,
            grammar,
            _marker: PhantomData,
        }
    }

    /// Erase the grammar type parameter, producing an [`AnyStatementResult`].
    ///
    /// Required when passing this result to [`GrammarNodeType::from_arena`],
    /// which expects a type-erased `stmt_result`.
    pub fn erase(self) -> AnyStatementResult<'a> {
        TypedStatementResult {
            raw: self.raw,
            source: self.source,
            grammar: self.grammar,
            _marker: PhantomData,
        }
    }

    /// The typed root node for this statement, or `None` if unavailable.
    pub fn root(&self) -> Option<G::Node<'a>> {
        let id = self.root_id();
        if id.is_null() {
            return None;
        }
        G::Node::from_arena(self.erase(), id)
    }

    /// The source text bound to this result.
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// Per-statement token positions. Requires `collect_tokens: true`.
    pub fn tokens(&self) -> &'a [TokenPos] {
        // SAFETY: self.raw is a valid, non-null parser pointer for lifetime 'a.
        unsafe { self.raw.as_ref().result_tokens() }
    }

    /// Per-statement comments. Requires `collect_tokens: true`.
    pub fn comments(&self) -> &'a [Comment] {
        // SAFETY: self.raw is a valid, non-null parser pointer for lifetime 'a.
        unsafe { self.raw.as_ref().result_comments() }
    }

    /// Whether this result has an associated parse error (RECOVERED case).
    pub fn has_error(&self) -> bool {
        // SAFETY: self.raw is a valid, non-null parser pointer for lifetime 'a.
        unsafe { self.raw.as_ref().result_error() != 0 }
    }

    // ── Result accessors (mirror syntaqlite_result_*) ──────────────────────

    /// Root node ID for the current statement (`AnyNodeId::NULL` if none).
    pub(crate) fn root_id(&self) -> AnyNodeId {
        // SAFETY: self.raw is a valid, non-null parser pointer for lifetime 'a.
        AnyNodeId(unsafe { self.raw.as_ref().result_root() })
    }

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

    // ── Arena access ───────────────────────────────────────────────────────

    /// Resolve a `AnyNodeId` to a typed reference, validating the tag.
    /// Returns `None` if null, invalid, or tag mismatch.
    pub(crate) fn resolve_as<T: ArenaNode>(&self, id: AnyNodeId) -> Option<&'a T> {
        let (ptr, tag) = self.node_ptr(id)?;
        if tag != T::TAG {
            return None;
        }
        // SAFETY: tag matches T::TAG, confirming the arena node has type T.
        // ptr is valid for 'a. T is #[repr(C)] with a u32 tag as its first
        // field, matching the arena layout.
        Some(unsafe { &*ptr.cast::<T>() })
    }

    /// Resolve a `AnyNodeId` as a [`NodeList`] (for list nodes).
    /// Returns `None` if null or invalid.
    pub(crate) fn resolve_list(&self, id: AnyNodeId) -> Option<&'a NodeList> {
        let (ptr, _) = self.node_ptr(id)?;
        // SAFETY: ptr is valid for 'a. List nodes have NodeList layout
        // (tag, count, children[count]). The caller is responsible for
        // ensuring the id refers to a list node (enforced by codegen).
        // The arena guarantees alignment of all allocated nodes.
        #[allow(clippy::cast_ptr_alignment)]
        Some(unsafe { &*ptr.cast::<NodeList>() })
    }

    /// Get a raw pointer to a node in the arena. Returns `(pointer, tag)`.
    pub(crate) fn node_ptr(&self, id: AnyNodeId) -> Option<(*const u8, u32)> {
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
            let tag = *ptr;
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

/// The result of a successfully parsed `SQLite` statement.
/// Produced by [`ParseSession::next_statement`].
#[cfg(feature = "sqlite")]
pub(crate) type StatementResult<'a> =
    TypedStatementResult<'a, crate::sqlite::grammar::SqliteGrammar>;

/// A type-erased statement result. The grammar type parameter is fixed to [`AnyDialect`].
pub(crate) type AnyStatementResult<'a> = TypedStatementResult<'a, AnyDialect>;

/// A parse error with human-readable message, optional source location, and
/// optionally a partial recovery tree. Parameterised by dialect grammar `G`.
///
/// Obtain via [`ParseSession::next_statement`] or
/// [`TypedParseSession::next_statement`].
pub(crate) struct TypedParseError<'a, G: TypedGrammar>(TypedStatementResult<'a, G>);

impl<'a, G: TypedGrammar> TypedParseError<'a, G> {
    pub(crate) fn new(result: TypedStatementResult<'a, G>) -> Self {
        TypedParseError(result)
    }

    pub(crate) fn message(&self) -> &str {
        self.0.error_msg().unwrap_or("parse error")
    }
    pub(crate) fn offset(&self) -> Option<usize> {
        self.0.error_offset()
    }
    pub(crate) fn length(&self) -> Option<usize> {
        self.0.error_length()
    }
    /// The partial recovery tree, if error recovery produced one.
    pub(crate) fn root(&self) -> Option<G::Node<'a>> {
        self.0.root()
    }
}

impl<G: TypedGrammar> std::fmt::Debug for TypedParseError<'_, G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedParseError")
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

/// A parse error from the `SQLite` dialect parser.
///
/// Obtain via [`ParseSession::next_statement`].
#[cfg(feature = "sqlite")]
pub struct ParseError<'a>(pub(crate) TypedParseError<'a, crate::sqlite::grammar::SqliteGrammar>);

#[cfg(feature = "sqlite")]
impl<'a> ParseError<'a> {
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

// ── Crate-internal ───────────────────────────────────────────────────────────

/// A source span describing where an error node was recorded in the arena.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ErrorSpan {
    pub(crate) offset: u32,
    pub(crate) length: u32,
}

/// Internal parse error value — lifetime-free, used by the incremental API.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct AnyParseError {
    pub(crate) message: String,
    pub(crate) offset: Option<usize>,
    pub(crate) length: Option<usize>,
    pub(crate) root: Option<AnyNodeId>,
}

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
    pub enum CCommentKind {
        LineComment = 0,
        BlockComment = 1,
    }

    /// Mirrors C `SyntaqliteComment`.
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct CComment {
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

    /// Mirrors C `SyntaqliteTokenPos`.
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct CTokenPos {
        pub offset: u32,
        pub length: u32,
        pub type_: u32,
        pub flags: u32,
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

        pub(crate) unsafe fn result_error(&self) -> u32 {
            // SAFETY: self is a valid, non-null CParser pointer; result
            // accessors are valid after `next()` returns a non-DONE code.
            unsafe { syntaqlite_result_error(std::ptr::from_ref::<Self>(self).cast_mut()) }
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

        pub(crate) unsafe fn result_tokens(&self) -> &[CTokenPos] {
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
            // SAFETY: ptr is a valid pointer to `count` CTokenPos values owned
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

        pub(crate) unsafe fn completion_context(&self) -> u32 {
            // SAFETY: self is a valid, non-null CParser pointer owned by the caller.
            unsafe {
                syntaqlite_parser_completion_context(std::ptr::from_ref::<Self>(self).cast_mut())
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
        fn syntaqlite_result_error(p: *mut CParser) -> u32;
        fn syntaqlite_result_error_msg(p: *mut CParser) -> *const c_char;
        fn syntaqlite_result_error_offset(p: *mut CParser) -> u32;
        fn syntaqlite_result_error_length(p: *mut CParser) -> u32;
        fn syntaqlite_result_comments(p: *mut CParser, count: *mut u32) -> *const CComment;
        fn syntaqlite_result_tokens(p: *mut CParser, count: *mut u32) -> *const CTokenPos;
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
        fn syntaqlite_parser_completion_context(p: *mut CParser) -> u32;
        fn syntaqlite_parser_begin_macro(p: *mut CParser, call_offset: u32, call_length: u32);
        fn syntaqlite_parser_end_macro(p: *mut CParser);
    }
}

pub(crate) use ffi::{
    CComment as Comment, CMacroRegion as MacroRegion, CParser, CTokenPos as TokenPos,
};

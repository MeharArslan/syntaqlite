use std::ffi::{c_int, CStr};

use crate::dialect::Dialect;
use super::ffi;
use super::ffi::{MacroRegion, Trivia};
use super::nodes::NodeId;

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
    raw: *mut ffi::Parser,
    /// Null-terminated copy of the source text. The C tokenizer (SQLite's
    /// `synq_sqlite3GetToken`) reads until it hits a null byte, so we must
    /// ensure the source is null-terminated. Rust `&str` does not guarantee
    /// this. The buffer is reused across `parse()` calls to avoid repeated
    /// allocations.
    source_buf: Vec<u8>,
}

// SAFETY: The C parser is self-contained (no thread-local or shared mutable
// state). Moving it between threads is safe; concurrent access is prevented
// by &mut borrowing in parse().
unsafe impl Send for Parser {}

impl Parser {
    pub fn new(dialect: &Dialect) -> Self {
        // SAFETY: syntaqlite_create_parser_with_dialect(NULL, dialect) allocates
        // a new parser with default malloc/free. It always succeeds.
        let raw = unsafe {
            ffi::syntaqlite_create_parser_with_dialect(std::ptr::null(), dialect.raw)
        };
        assert!(!raw.is_null(), "parser allocation failed");
        Parser {
            raw,
            source_buf: Vec::new(),
        }
    }

    /// Enable Lemon trace output to stderr (debug builds only).
    pub fn set_trace(&mut self, enable: bool) {
        // SAFETY: self.raw is valid for the lifetime of Parser (Drop cleans up).
        unsafe { ffi::syntaqlite_parser_set_trace(self.raw, enable as _) }
    }

    /// Enable token/trivia collection. Required for comment preservation
    /// during formatting.
    pub fn set_collect_tokens(&mut self, enable: bool) {
        unsafe { ffi::syntaqlite_parser_set_collect_tokens(self.raw, enable as c_int) }
    }

    /// Shared session initialization: copies source, resets the C parser,
    /// returns a `SessionBase`.
    fn init_session<'a>(&'a mut self, source: &'a str) -> SessionBase<'a> {
        self.source_buf.clear();
        self.source_buf.reserve(source.len() + 1);
        self.source_buf.extend_from_slice(source.as_bytes());
        self.source_buf.push(0);

        let c_source_ptr = self.source_buf.as_ptr();
        unsafe {
            ffi::syntaqlite_parser_reset(
                self.raw,
                c_source_ptr as *const _,
                source.len() as u32,
            );
        }
        SessionBase {
            parser: self,
            source,
            c_source_ptr,
        }
    }

    /// Shared session initialization from a CStr (zero-copy).
    fn init_session_cstr<'a>(&'a mut self, source: &'a CStr) -> SessionBase<'a> {
        let bytes = source.to_bytes();
        let source_str =
            std::str::from_utf8(bytes).expect("source must be valid UTF-8");

        unsafe {
            ffi::syntaqlite_parser_reset(
                self.raw,
                source.as_ptr(),
                bytes.len() as u32,
            );
        }
        SessionBase {
            parser: self,
            source: source_str,
            c_source_ptr: source.as_ptr() as *const u8,
        }
    }

    /// Bind source text and return a `Session` for iterating statements.
    ///
    /// Copies the source into an internal buffer to add a null terminator
    /// (required by the C tokenizer). For zero-copy parsing, use
    /// [`parse_cstr`](Self::parse_cstr).
    pub fn parse<'a>(&'a mut self, source: &'a str) -> Session<'a> {
        Session(self.init_session(source))
    }

    /// Zero-copy variant: bind a null-terminated source and return a `Session`.
    ///
    /// The `&CStr` already guarantees a trailing `\0`, so no copy is needed.
    /// The source must be valid UTF-8 (panics otherwise).
    pub fn parse_cstr<'a>(&'a mut self, source: &'a CStr) -> Session<'a> {
        Session(self.init_session_cstr(source))
    }

    /// Bind source text and return a `TokenSession` for low-level token feeding.
    pub fn token_session<'a>(&'a mut self, source: &'a str) -> TokenSession<'a> {
        TokenSession(self.init_session(source))
    }

    /// Zero-copy variant: bind a null-terminated source and return a `TokenSession`.
    pub fn token_session_cstr<'a>(&'a mut self, source: &'a CStr) -> TokenSession<'a> {
        TokenSession(self.init_session_cstr(source))
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

// ── SessionBase ─────────────────────────────────────────────────────────

/// Shared read-only session state. Both `Session` and `TokenSession` wrap this.
pub struct SessionBase<'a> {
    parser: &'a mut Parser,
    source: &'a str,
    /// The pointer that the C parser uses as its source base. This may differ
    /// from `source.as_ptr()` when `parse()` copies into an internal buffer.
    /// `feed_token` translates user text pointers through this so that the C
    /// code's `tok.z - ctx->source` offset arithmetic is correct regardless
    /// of whether the copying or zero-copy path was used.
    c_source_ptr: *const u8,
}

impl<'a> SessionBase<'a> {
    /// Get a raw pointer to a node in the arena. Returns (pointer, tag).
    ///
    /// This is the dialect-agnostic primitive. Dialect crates wrap this to
    /// return a typed `Node` enum.
    pub fn node_ptr(&self, id: NodeId) -> Option<(*const u8, u32)> {
        if id.is_null() {
            return None;
        }
        // SAFETY: parser.raw is valid. syntaqlite_parser_node returns a pointer
        // into the arena that is valid until the next reset() or destroy(), both
        // of which require &mut access that the session holds exclusively.
        let ptr = unsafe { ffi::syntaqlite_parser_node(self.parser.raw, id.0) };
        if ptr.is_null() {
            return None;
        }
        let tag = unsafe { *(ptr as *const u32) };
        Some((ptr as *const u8, tag))
    }

    /// The source text bound to this session.
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// Return all trivia (comments) captured during parsing.
    /// Requires `set_collect_tokens(true)` before parsing.
    ///
    /// Returns a slice into the parser's internal buffer — valid until
    /// the parser is reset or destroyed (which requires `&mut`).
    pub fn trivia(&self) -> &[Trivia] {
        let mut count: u32 = 0;
        let ptr = unsafe { ffi::syntaqlite_parser_trivia(self.parser.raw, &mut count) };
        if count == 0 || ptr.is_null() {
            return &[];
        }
        // SAFETY: The pointer is valid for the lifetime of &self.
        unsafe { std::slice::from_raw_parts(ptr, count as usize) }
    }

    /// Dump an AST node tree as indented text. Uses C-side metadata (field
    /// names, display strings) so no Rust-side string tables are needed.
    pub fn dump_node(&self, id: NodeId, out: &mut String, indent: usize) {
        let ptr = unsafe {
            ffi::syntaqlite_dump_node(self.parser.raw, id.0, indent as u32)
        };
        if !ptr.is_null() {
            let cstr = unsafe { CStr::from_ptr(ptr) };
            out.push_str(&cstr.to_string_lossy());
            // Free the C-allocated string via the parser's default allocator (free).
            unsafe extern "C" { fn free(ptr: *mut std::ffi::c_void); }
            unsafe { free(ptr as *mut std::ffi::c_void) };
        }
    }

    /// Return all macro regions recorded via `begin_macro`/`end_macro`.
    pub fn macro_regions(&self) -> &[MacroRegion] {
        let mut count: u32 = 0;
        let ptr =
            unsafe { ffi::syntaqlite_parser_macro_regions(self.parser.raw, &mut count) };
        if count == 0 || ptr.is_null() {
            return &[];
        }
        // SAFETY: The pointer is valid for the lifetime of &self.
        unsafe { std::slice::from_raw_parts(ptr, count as usize) }
    }
}

// ── Session (high-level) ────────────────────────────────────────────────

/// A high-level parsing session. Iterate with `next_statement()`.
pub struct Session<'a>(pub(crate) SessionBase<'a>);

impl<'a> Session<'a> {
    /// Parse the next SQL statement. Returns `None` when all statements have
    /// been consumed (or input was empty).
    pub fn next_statement(&mut self) -> Option<Result<NodeId, ParseError>> {
        // SAFETY: parser.raw is valid and exclusively borrowed via &mut self.
        let result = unsafe { ffi::syntaqlite_parser_next(self.0.parser.raw) };

        let id = NodeId(result.root);
        if !id.is_null() {
            return Some(Ok(id));
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

    /// Access the underlying `SessionBase` for read-only operations.
    pub fn base(&self) -> &SessionBase<'a> {
        &self.0
    }

    // Delegate read-only methods for convenience

    /// Get a raw pointer to a node in the arena. Returns (pointer, tag).
    pub fn node_ptr(&self, id: NodeId) -> Option<(*const u8, u32)> {
        self.0.node_ptr(id)
    }

    /// The source text bound to this session.
    pub fn source(&self) -> &'a str {
        self.0.source()
    }

    /// Return all trivia (comments) captured during parsing.
    pub fn trivia(&self) -> &[Trivia] {
        self.0.trivia()
    }

    /// Dump an AST node tree as indented text.
    pub fn dump_node(&self, id: NodeId, out: &mut String, indent: usize) {
        self.0.dump_node(id, out, indent)
    }

    /// Return all macro regions recorded via `begin_macro`/`end_macro`.
    pub fn macro_regions(&self) -> &[MacroRegion] {
        self.0.macro_regions()
    }
}

impl Iterator for Session<'_> {
    type Item = Result<NodeId, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_statement()
    }
}

// ── TokenSession (low-level) ────────────────────────────────────────────

/// A low-level token-feeding session. Feed tokens manually and get parse results.
pub struct TokenSession<'a>(pub(crate) SessionBase<'a>);

impl<'a> TokenSession<'a> {
    /// Feed a single token to the parser.
    ///
    /// `TK_SPACE` is silently skipped. `TK_COMMENT` is recorded as trivia
    /// (when `set_collect_tokens` is enabled) but not fed to the parser.
    ///
    /// Returns `Ok(Some(root_id))` when a statement completes,
    /// `Ok(None)` to keep going, or `Err` on parse error.
    ///
    /// The `text` pointer must point into the source buffer bound by the session.
    /// `token_type` is a raw token type ordinal (dialect-specific).
    pub fn feed_token(
        &mut self,
        token_type: u32,
        text: &str,
    ) -> Result<Option<NodeId>, ParseError> {
        // Translate the text pointer so it's relative to the C parser's source
        // buffer.
        let offset = text.as_ptr() as usize - self.0.source.as_ptr() as usize;
        let c_text = unsafe { self.0.c_source_ptr.add(offset) };
        let rc = unsafe {
            ffi::syntaqlite_parser_feed_token(
                self.0.parser.raw,
                token_type as c_int,
                c_text as *const _,
                text.len() as c_int,
            )
        };
        match rc {
            0 => Ok(None),
            1 => {
                let result = unsafe { ffi::syntaqlite_parser_result(self.0.parser.raw) };
                Ok(Some(NodeId(result.root)))
            }
            _ => {
                let result = unsafe { ffi::syntaqlite_parser_result(self.0.parser.raw) };
                let msg = if result.error_msg.is_null() {
                    "parse error".to_string()
                } else {
                    unsafe { CStr::from_ptr(result.error_msg) }
                        .to_string_lossy()
                        .into_owned()
                };
                Err(ParseError { message: msg })
            }
        }
    }

    /// Signal end of input when using the low-level token-feeding API.
    ///
    /// Synthesizes a SEMI if the last token wasn't one, and sends EOF to
    /// the parser. Returns `Ok(Some(root_id))` if a final statement
    /// completed, `Ok(None)` if there was nothing pending, or `Err` on
    /// parse error.
    pub fn finish(&mut self) -> Result<Option<NodeId>, ParseError> {
        let rc = unsafe { ffi::syntaqlite_parser_finish(self.0.parser.raw) };
        match rc {
            0 => Ok(None),
            1 => {
                let result = unsafe { ffi::syntaqlite_parser_result(self.0.parser.raw) };
                Ok(Some(NodeId(result.root)))
            }
            _ => {
                let result = unsafe { ffi::syntaqlite_parser_result(self.0.parser.raw) };
                let msg = if result.error_msg.is_null() {
                    "parse error".to_string()
                } else {
                    unsafe { CStr::from_ptr(result.error_msg) }
                        .to_string_lossy()
                        .into_owned()
                };
                Err(ParseError { message: msg })
            }
        }
    }

    /// Mark subsequent fed tokens as being inside a macro expansion.
    ///
    /// `call_offset` and `call_length` describe the macro call's byte range
    /// in the original source. Calls may nest (for nested macro expansions).
    pub fn begin_macro(&mut self, call_offset: u32, call_length: u32) {
        unsafe {
            ffi::syntaqlite_parser_begin_macro(self.0.parser.raw, call_offset, call_length);
        }
    }

    /// End the innermost macro expansion region.
    pub fn end_macro(&mut self) {
        unsafe {
            ffi::syntaqlite_parser_end_macro(self.0.parser.raw);
        }
    }

    /// Access the underlying `SessionBase` for read-only operations.
    pub fn base(&self) -> &SessionBase<'a> {
        &self.0
    }

    // Delegate read-only methods for convenience

    /// Get a raw pointer to a node in the arena. Returns (pointer, tag).
    pub fn node_ptr(&self, id: NodeId) -> Option<(*const u8, u32)> {
        self.0.node_ptr(id)
    }

    /// The source text bound to this session.
    pub fn source(&self) -> &'a str {
        self.0.source()
    }

    /// Return all trivia (comments) captured during parsing.
    pub fn trivia(&self) -> &[Trivia] {
        self.0.trivia()
    }

    /// Dump an AST node tree as indented text.
    pub fn dump_node(&self, id: NodeId, out: &mut String, indent: usize) {
        self.0.dump_node(id, out, indent)
    }

    /// Return all macro regions recorded via `begin_macro`/`end_macro`.
    pub fn macro_regions(&self) -> &[MacroRegion] {
        self.0.macro_regions()
    }
}

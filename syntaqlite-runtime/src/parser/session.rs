use std::ffi::{c_int, CStr};

use super::ffi;
use super::nodes::NodeId;

// Compile-time check: Trivia must have identical layout to ffi::RawTrivia
// so we can transmute the C pointer directly to &[Trivia].
const _: () = {
    assert!(std::mem::size_of::<Trivia>() == std::mem::size_of::<ffi::RawTrivia>());
    assert!(std::mem::align_of::<Trivia>() == std::mem::align_of::<ffi::RawTrivia>());
    assert!(std::mem::offset_of!(Trivia, offset) == std::mem::offset_of!(ffi::RawTrivia, offset));
    assert!(std::mem::offset_of!(Trivia, length) == std::mem::offset_of!(ffi::RawTrivia, length));
    assert!(std::mem::offset_of!(Trivia, kind) == std::mem::offset_of!(ffi::RawTrivia, kind));
};

// Compile-time check: MacroRegion must have identical layout to ffi::RawMacroRegion.
const _: () = {
    assert!(std::mem::size_of::<MacroRegion>() == std::mem::size_of::<ffi::RawMacroRegion>());
    assert!(std::mem::align_of::<MacroRegion>() == std::mem::align_of::<ffi::RawMacroRegion>());
    assert!(
        std::mem::offset_of!(MacroRegion, call_offset)
            == std::mem::offset_of!(ffi::RawMacroRegion, call_offset)
    );
    assert!(
        std::mem::offset_of!(MacroRegion, call_length)
            == std::mem::offset_of!(ffi::RawMacroRegion, call_length)
    );
};

/// The kind of a trivia item (comment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TriviaKind {
    /// A line comment starting with `--`.
    LineComment = 0,
    /// A block comment delimited by `/* ... */`.
    BlockComment = 1,
}

/// A comment captured during parsing. Trivia items are sorted by source offset.
///
/// Layout matches `SyntaqliteTrivia` in C (offset: u32, length: u32, kind: u8)
/// so we can return a slice directly from the C buffer without copying.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Trivia {
    pub offset: u32,
    pub length: u32,
    pub kind: TriviaKind,
}

/// A recorded macro invocation region. Populated via the low-level API
/// (`begin_macro` / `end_macro`). The formatter can use these to reconstruct
/// macro calls from the expanded AST.
///
/// Layout matches `SyntaqliteMacroRegion` in C.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MacroRegion {
    /// Byte offset of the macro call in the original source.
    pub call_offset: u32,
    /// Byte length of the entire macro call.
    pub call_length: u32,
}

/// An opaque dialect handle. Dialect crates (e.g. `syntaqlite`) provide a
/// function that returns a `&'static Dialect` for their grammar.
pub struct Dialect {
    raw: *const ffi::RawDialect,
}

impl Dialect {
    /// Create a `Dialect` from a raw C pointer returned by a dialect's
    /// FFI function (e.g. `syntaqlite_sqlite_dialect`).
    ///
    /// # Safety
    /// The pointer must point to a valid `SynqDialect` with `'static` lifetime.
    pub unsafe fn from_raw(raw: *const std::ffi::c_void) -> Self {
        Dialect {
            raw: raw as *const ffi::RawDialect,
        }
    }
}

// SAFETY: The dialect is a pointer to a static C struct with no mutable state.
unsafe impl Send for Dialect {}
unsafe impl Sync for Dialect {}

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

    /// Bind source text and return a `Session` for iterating statements.
    ///
    /// Copies the source into an internal buffer to add a null terminator
    /// (required by the C tokenizer). For zero-copy parsing, use
    /// [`parse_cstr`](Self::parse_cstr).
    pub fn parse<'a>(&'a mut self, source: &'a str) -> Session<'a> {
        self.source_buf.clear();
        self.source_buf.reserve(source.len() + 1);
        self.source_buf.extend_from_slice(source.as_bytes());
        self.source_buf.push(0);

        // SAFETY: source_buf is null-terminated and lives as long as
        // the Parser. The Session borrows &mut self, preventing
        // mutation until it is dropped.
        let c_source_ptr = self.source_buf.as_ptr();
        unsafe {
            ffi::syntaqlite_parser_reset(
                self.raw,
                c_source_ptr as *const _,
                source.len() as u32,
            );
        }
        Session {
            parser: self,
            source,
            c_source_ptr,
        }
    }

    /// Zero-copy variant: bind a null-terminated source and return a `Session`.
    ///
    /// The `&CStr` already guarantees a trailing `\0`, so no copy is needed.
    /// The source must be valid UTF-8 (panics otherwise).
    pub fn parse_cstr<'a>(&'a mut self, source: &'a CStr) -> Session<'a> {
        let bytes = source.to_bytes(); // excludes the null terminator
        let source_str =
            std::str::from_utf8(bytes).expect("source must be valid UTF-8");

        // SAFETY: CStr guarantees null-termination. The borrow on `source`
        // in Session<'a> keeps the pointer valid for the session's lifetime.
        unsafe {
            ffi::syntaqlite_parser_reset(
                self.raw,
                source.as_ptr(),
                bytes.len() as u32,
            );
        }
        Session {
            parser: self,
            source: source_str,
            c_source_ptr: source.as_ptr() as *const u8,
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

/// A parsing session tied to a source string. Iterate with `next_statement()`.
pub struct Session<'a> {
    parser: &'a mut Parser,
    source: &'a str,
    /// The pointer that the C parser uses as its source base. This may differ
    /// from `source.as_ptr()` when `parse()` copies into an internal buffer.
    /// `feed_token` translates user text pointers through this so that the C
    /// code's `tok.z - ctx->source` offset arithmetic is correct regardless
    /// of whether the copying or zero-copy path was used.
    c_source_ptr: *const u8,
}

impl<'a> Session<'a> {
    /// Parse the next SQL statement. Returns `None` when all statements have
    /// been consumed (or input was empty).
    pub fn next_statement(&mut self) -> Option<Result<NodeId, ParseError>> {
        // SAFETY: parser.raw is valid and exclusively borrowed via &mut self.
        let result = unsafe { ffi::syntaqlite_parser_next(self.parser.raw) };

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
        // of which require &mut access that Session holds exclusively.
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
        // SAFETY: RawTrivia and Trivia have identical repr(C) layout
        // (u32, u32, u8). The pointer is valid for the lifetime of &self.
        unsafe { std::slice::from_raw_parts(ptr as *const Trivia, count as usize) }
    }

    // -- Low-level token-feeding API ------------------------------------------

    /// Feed a single token to the parser.
    ///
    /// `TK_SPACE` is silently skipped. `TK_COMMENT` is recorded as trivia
    /// (when `set_collect_tokens` is enabled) but not fed to the parser.
    ///
    /// Returns `Ok(Some(root_id))` when a statement completes,
    /// `Ok(None)` to keep going, or `Err` on parse error.
    ///
    /// The `text` pointer must point into the source buffer bound by `parse()`.
    /// `token_type` is a raw token type ordinal (dialect-specific).
    pub fn feed_token(
        &mut self,
        token_type: u32,
        text: &str,
    ) -> Result<Option<NodeId>, ParseError> {
        // Translate the text pointer so it's relative to the C parser's source
        // buffer. When parse() copies the source into an internal buffer, the
        // user's text slice points into the original string while the C parser
        // expects pointers into the copy. Computing the byte offset within
        // self.source and adding it to c_source_ptr makes the C-side
        // `tok.z - ctx->source` arithmetic correct in both the copying
        // (parse) and zero-copy (parse_cstr) paths.
        let offset = text.as_ptr() as usize - self.source.as_ptr() as usize;
        let c_text = unsafe { self.c_source_ptr.add(offset) };
        let rc = unsafe {
            ffi::syntaqlite_parser_feed_token(
                self.parser.raw,
                token_type as c_int,
                c_text as *const _,
                text.len() as c_int,
            )
        };
        match rc {
            0 => Ok(None),
            1 => {
                let result = unsafe { ffi::syntaqlite_parser_result(self.parser.raw) };
                Ok(Some(NodeId(result.root)))
            }
            _ => {
                let result = unsafe { ffi::syntaqlite_parser_result(self.parser.raw) };
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
        let rc = unsafe { ffi::syntaqlite_parser_finish(self.parser.raw) };
        match rc {
            0 => Ok(None),
            1 => {
                let result = unsafe { ffi::syntaqlite_parser_result(self.parser.raw) };
                Ok(Some(NodeId(result.root)))
            }
            _ => {
                let result = unsafe { ffi::syntaqlite_parser_result(self.parser.raw) };
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
            ffi::syntaqlite_parser_begin_macro(self.parser.raw, call_offset, call_length);
        }
    }

    /// End the innermost macro expansion region.
    pub fn end_macro(&mut self) {
        unsafe {
            ffi::syntaqlite_parser_end_macro(self.parser.raw);
        }
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
        // SAFETY: RawMacroRegion and MacroRegion have identical repr(C) layout.
        unsafe { std::slice::from_raw_parts(ptr as *const MacroRegion, count as usize) }
    }
}

impl Iterator for Session<'_> {
    type Item = Result<NodeId, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_statement()
    }
}

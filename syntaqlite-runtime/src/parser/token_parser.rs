use std::ffi::{c_int, CStr};
use std::ops::Range;

use crate::dialect::Dialect;
use super::ffi;
use super::ffi::{MacroRegion, Trivia};
use super::nodes::NodeId;
use super::parser::{CursorBase, ParseError, ParserConfig};

/// A low-level parser for token-by-token feeding. Owns its own C parser
/// handle and source buffer, independent of `Parser`.
pub struct TokenParser {
    raw: *mut ffi::Parser,
    source_buf: Vec<u8>,
}

// SAFETY: Same reasoning as Parser — the C parser is self-contained.
unsafe impl Send for TokenParser {}

impl TokenParser {
    /// Create a new token parser for the given dialect.
    pub fn new(dialect: &Dialect) -> Self {
        let raw = unsafe {
            ffi::syntaqlite_create_parser_with_dialect(std::ptr::null(), dialect.raw)
        };
        assert!(!raw.is_null(), "parser allocation failed");
        TokenParser {
            raw,
            source_buf: Vec::new(),
        }
    }

    /// Create a token parser with the given configuration.
    pub fn with_config(dialect: &Dialect, config: &ParserConfig) -> Self {
        let tp = Self::new(dialect);
        unsafe {
            ffi::syntaqlite_parser_set_trace(tp.raw, config.trace as c_int);
            ffi::syntaqlite_parser_set_collect_tokens(tp.raw, config.collect_tokens as c_int);
        }
        tp
    }

    /// Enable or disable parser trace output.
    pub fn set_trace(&mut self, enable: bool) {
        unsafe {
            ffi::syntaqlite_parser_set_trace(self.raw, enable as c_int);
        }
    }

    /// Enable or disable token collection (needed for trivia capture).
    pub fn set_collect_tokens(&mut self, enable: bool) {
        unsafe {
            ffi::syntaqlite_parser_set_collect_tokens(self.raw, enable as c_int);
        }
    }

    /// Bind source text and return a `TokenFeeder` for low-level token feeding.
    pub fn feed<'a>(&'a mut self, source: &'a str) -> TokenFeeder<'a> {
        let base = CursorBase::new(self.raw, &mut self.source_buf, source);
        TokenFeeder(base)
    }

    /// Zero-copy variant: bind a null-terminated source and return a `TokenFeeder`.
    pub fn feed_cstr<'a>(&'a mut self, source: &'a CStr) -> TokenFeeder<'a> {
        let base = CursorBase::new_cstr(self.raw, source);
        TokenFeeder(base)
    }
}

impl Drop for TokenParser {
    fn drop(&mut self) {
        unsafe { ffi::syntaqlite_parser_destroy(self.raw) }
    }
}

// ── TokenFeeder ─────────────────────────────────────────────────────────

/// A low-level token-feeding cursor. Feed tokens manually and get parse results.
pub struct TokenFeeder<'a>(pub(crate) CursorBase<'a>);

impl<'a> TokenFeeder<'a> {
    /// Read the parser result after a statement-completing or error return code.
    /// rc == 1 means success, anything else is an error.
    fn parse_result(&self, rc: c_int) -> Result<NodeId, ParseError> {
        // SAFETY: raw is valid; result struct and error_msg pointer are valid
        // for the lifetime of the parser.
        unsafe {
            let result = ffi::syntaqlite_parser_result(self.0.raw);
            if rc == 1 {
                return Ok(NodeId(result.root));
            }
            let msg = if result.error_msg.is_null() {
                "parse error".to_string()
            } else {
                CStr::from_ptr(result.error_msg)
                    .to_string_lossy()
                    .into_owned()
            };
            Err(ParseError { message: msg })
        }
    }

    /// Feed a single token to the parser.
    ///
    /// `TK_SPACE` is silently skipped. `TK_COMMENT` is recorded as trivia
    /// (when `collect_tokens` is enabled) but not fed to the parser.
    ///
    /// Returns `Ok(Some(root_id))` when a statement completes,
    /// `Ok(None)` to keep going, or `Err` on parse error.
    ///
    /// `span` is a byte range into the source text bound by this feeder.
    /// `token_type` is a raw token type ordinal (dialect-specific).
    pub fn feed_token(
        &mut self,
        token_type: u32,
        span: Range<usize>,
    ) -> Result<Option<NodeId>, ParseError> {
        // SAFETY: c_source_ptr is valid for the source length; raw is valid.
        let rc = unsafe {
            let c_text = self.0.c_source_ptr.add(span.start);
            ffi::syntaqlite_parser_feed_token(
                self.0.raw,
                token_type as c_int,
                c_text as *const _,
                span.len() as c_int,
            )
        };
        match rc {
            0 => Ok(None),
            _ => self.parse_result(rc).map(Some),
        }
    }

    /// Signal end of input when using the low-level token-feeding API.
    ///
    /// Synthesizes a SEMI if the last token wasn't one, and sends EOF to
    /// the parser. Returns `Ok(Some(root_id))` if a final statement
    /// completed, `Ok(None)` if there was nothing pending, or `Err` on
    /// parse error.
    pub fn finish(&mut self) -> Result<Option<NodeId>, ParseError> {
        // SAFETY: raw is valid.
        let rc = unsafe { ffi::syntaqlite_parser_finish(self.0.raw) };
        match rc {
            0 => Ok(None),
            _ => self.parse_result(rc).map(Some),
        }
    }

    /// Mark subsequent fed tokens as being inside a macro expansion.
    ///
    /// `call_offset` and `call_length` describe the macro call's byte range
    /// in the original source. Calls may nest (for nested macro expansions).
    pub fn begin_macro(&mut self, call_offset: u32, call_length: u32) {
        unsafe {
            ffi::syntaqlite_parser_begin_macro(self.0.raw, call_offset, call_length);
        }
    }

    /// End the innermost macro expansion region.
    pub fn end_macro(&mut self) {
        unsafe {
            ffi::syntaqlite_parser_end_macro(self.0.raw);
        }
    }

    /// Access the underlying `CursorBase` for read-only operations.
    pub fn base(&self) -> &CursorBase<'a> {
        &self.0
    }

    // Delegate read-only methods for convenience

    /// Get a raw pointer to a node in the arena. Returns (pointer, tag).
    pub fn node_ptr(&self, id: NodeId) -> Option<(*const u8, u32)> {
        self.0.node_ptr(id)
    }

    /// The source text bound to this feeder.
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

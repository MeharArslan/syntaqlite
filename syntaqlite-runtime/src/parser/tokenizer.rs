use super::ffi;

/// A raw token: (token_type ordinal, text slice).
#[derive(Debug, Clone, Copy)]
pub struct RawToken<'a> {
    /// The token type as a raw `u32` ordinal (dialect-specific).
    pub token_type: u32,
    /// The text of the token (a slice of the source).
    pub text: &'a str,
}

/// Owns a tokenizer instance. Reusable across inputs via `tokenize()`.
pub struct Tokenizer {
    raw: *mut ffi::Tokenizer,
    /// Null-terminated copy of the source text.
    source_buf: Vec<u8>,
}

// SAFETY: The C tokenizer is self-contained (no thread-local or shared mutable
// state). Moving it between threads is safe; concurrent access is prevented
// by &mut borrowing in tokenize().
unsafe impl Send for Tokenizer {}

impl Tokenizer {
    /// Create a new tokenizer.
    pub fn new() -> Self {
        let raw = unsafe { ffi::syntaqlite_tokenizer_create(std::ptr::null()) };
        assert!(!raw.is_null(), "tokenizer allocation failed");
        Tokenizer {
            raw,
            source_buf: Vec::new(),
        }
    }

    /// Bind source text and return a `TokenizerSession` for iterating tokens.
    ///
    /// Copies the source into an internal buffer to add a null terminator
    /// (required by the C tokenizer).
    pub fn tokenize<'a>(&'a mut self, source: &'a str) -> TokenizerSession<'a> {
        self.source_buf.clear();
        self.source_buf.reserve(source.len() + 1);
        self.source_buf.extend_from_slice(source.as_bytes());
        self.source_buf.push(0);

        let c_source_ptr = self.source_buf.as_ptr();
        unsafe {
            ffi::syntaqlite_tokenizer_reset(
                self.raw,
                c_source_ptr as *const _,
                source.len() as u32,
            );
        }
        TokenizerSession {
            tokenizer: self,
            source,
        }
    }
}

impl Drop for Tokenizer {
    fn drop(&mut self) {
        unsafe { ffi::syntaqlite_tokenizer_destroy(self.raw) }
    }
}

/// An active tokenizer session. Iterates tokens from the bound source.
pub struct TokenizerSession<'a> {
    tokenizer: &'a mut Tokenizer,
    source: &'a str,
}

impl<'a> Iterator for TokenizerSession<'a> {
    type Item = RawToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut token = ffi::Token {
            text: std::ptr::null(),
            length: 0,
            type_: 0,
        };
        let rc = unsafe { ffi::syntaqlite_tokenizer_next(self.tokenizer.raw, &mut token) };
        if rc == 0 {
            return None;
        }

        // Compute offset into the source string from the C pointer.
        let c_base = self.tokenizer.source_buf.as_ptr();
        let offset = token.text as usize - c_base as usize;
        let len = token.length as usize;
        let text = &self.source[offset..offset + len];

        Some(RawToken {
            token_type: token.type_,
            text,
        })
    }
}

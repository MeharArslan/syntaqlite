use super::ffi;

/// A single token with its raw type ordinal, text, and byte offset.
#[derive(Debug, Clone)]
pub struct RawToken<'a> {
    pub text: &'a str,
    /// Raw token type ordinal. Dialect crates wrap this to provide typed access.
    pub token_type: u32,
    pub offset: u32,
}

/// Owns a tokenizer instance. Reusable across inputs via `tokenize()`.
pub struct RawTokenizer {
    raw: *mut ffi::RawTokenizer,
}

// SAFETY: Same rationale as Parser — the C tokenizer is self-contained.
unsafe impl Send for RawTokenizer {}

impl RawTokenizer {
    pub fn new() -> Self {
        // SAFETY: syntaqlite_tokenizer_create(NULL) allocates with default
        // malloc/free. Always succeeds (assert guards).
        let raw = unsafe { ffi::syntaqlite_tokenizer_create(std::ptr::null()) };
        assert!(!raw.is_null(), "tokenizer allocation failed");
        RawTokenizer { raw }
    }

    /// Bind source text and return a `RawTokenStream` iterator.
    pub fn tokenize<'a>(&'a mut self, source: &'a str) -> RawTokenStream<'a> {
        // SAFETY: self.raw is valid. source.as_ptr() is valid for source.len()
        // bytes. The borrow on `source` in RawTokenStream<'a> keeps it alive.
        unsafe {
            ffi::syntaqlite_tokenizer_reset(
                self.raw,
                source.as_ptr() as *const _,
                source.len() as u32,
            );
        }
        RawTokenStream {
            tokenizer: self,
            source,
        }
    }
}

impl Drop for RawTokenizer {
    fn drop(&mut self) {
        // SAFETY: self.raw was allocated by syntaqlite_tokenizer_create and
        // has not been freed. The C function is no-op on NULL.
        unsafe { ffi::syntaqlite_tokenizer_destroy(self.raw) }
    }
}

impl Default for RawTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

/// An iterator over tokens in a source string, yielding raw token type ordinals.
pub struct RawTokenStream<'a> {
    tokenizer: &'a mut RawTokenizer,
    source: &'a str,
}

impl<'a> Iterator for RawTokenStream<'a> {
    type Item = RawToken<'a>;

    fn next(&mut self) -> Option<RawToken<'a>> {
        let mut raw_token = ffi::RawToken {
            text: std::ptr::null(),
            length: 0,
            type_: 0,
        };
        // SAFETY: tokenizer.raw is valid and exclusively borrowed. raw_token
        // is a valid out-parameter. Returns 1 if a token was written, 0 at EOF.
        let ok = unsafe { ffi::syntaqlite_tokenizer_next(self.tokenizer.raw, &mut raw_token) };
        if ok == 0 {
            return None;
        }
        // SAFETY: On success (ok=1), raw_token.text points into the source
        // buffer bound by the last reset() call, which is self.source.
        // offset_from is valid because both pointers are within the same
        // allocation (the source string).
        let offset =
            unsafe { raw_token.text.offset_from(self.source.as_ptr() as *const _) as u32 };
        let text = &self.source[offset as usize..(offset as usize + raw_token.length as usize)];
        Some(RawToken {
            text,
            token_type: raw_token.type_,
            offset,
        })
    }
}

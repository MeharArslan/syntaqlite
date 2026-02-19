use syntaqlite_runtime::{RawTokenizer, RawTokenStream};
use crate::generated::tokens::TokenType;

/// A single token with its type, text, and byte offset.
#[derive(Debug, Clone)]
pub struct Token<'a> {
    pub text: &'a str,
    pub token_type: TokenType,
    pub offset: u32,
}

/// Owns a tokenizer instance. Reusable across inputs via `tokenize()`.
pub struct Tokenizer {
    inner: RawTokenizer,
}

impl Tokenizer {
    pub fn new() -> Self {
        Tokenizer {
            inner: RawTokenizer::new(),
        }
    }

    /// Bind source text and return a `TokenStream` iterator.
    pub fn tokenize<'a>(&'a mut self, source: &'a str) -> TokenStream<'a> {
        TokenStream {
            inner: self.inner.tokenize(source),
        }
    }
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::new()
    }
}

/// An iterator over tokens in a source string.
pub struct TokenStream<'a> {
    inner: RawTokenStream<'a>,
}

impl<'a> Iterator for TokenStream<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Token<'a>> {
        let raw = self.inner.next()?;
        let token_type = TokenType::from_raw(raw.token_type).unwrap_or(TokenType::Illegal);
        Some(Token {
            text: raw.text,
            token_type,
            offset: raw.offset,
        })
    }
}

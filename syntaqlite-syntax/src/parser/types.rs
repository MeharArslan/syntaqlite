// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use crate::grammar::TypedGrammar;

/// Tri-state parse result for statement-oriented parser APIs.
///
/// Mirrors C parser return codes:
/// - [`ParseOutcome::Done`]  -> `SYNTAQLITE_PARSE_DONE`
/// - [`ParseOutcome::Ok`]    -> `SYNTAQLITE_PARSE_OK`
/// - [`ParseOutcome::Err`]   -> `SYNTAQLITE_PARSE_ERROR`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseOutcome<T, E> {
    /// No more statements/results are available.
    Done,
    /// A statement parsed successfully.
    Ok(T),
    /// A statement parsed with an error.
    Err(E),
}

impl<T, E> ParseOutcome<T, E> {
    /// Convert into `Result<Option<T>, E>` for `?`-friendly control flow.
    ///
    /// # Errors
    ///
    /// Returns `Err(e)` when the outcome is [`ParseOutcome::Err`].
    pub fn transpose(self) -> Result<Option<T>, E> {
        match self {
            ParseOutcome::Done => Ok(None),
            ParseOutcome::Ok(v) => Ok(Some(v)),
            ParseOutcome::Err(e) => Err(e),
        }
    }

    /// Map the `Ok(T)` payload.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> ParseOutcome<U, E> {
        match self {
            ParseOutcome::Done => ParseOutcome::Done,
            ParseOutcome::Ok(v) => ParseOutcome::Ok(f(v)),
            ParseOutcome::Err(e) => ParseOutcome::Err(e),
        }
    }

    /// Map the `Err(E)` payload.
    pub fn map_err<F>(self, f: impl FnOnce(E) -> F) -> ParseOutcome<T, F> {
        match self {
            ParseOutcome::Done => ParseOutcome::Done,
            ParseOutcome::Ok(v) => ParseOutcome::Ok(v),
            ParseOutcome::Err(e) => ParseOutcome::Err(f(e)),
        }
    }
}

/// SQL comment style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentKind {
    /// A line comment starting with `--`.
    Line,
    /// A block comment delimited by `/* ... */`.
    Block,
}

/// Comment captured from source during parsing.
///
/// Returned by [`super::TypedParsedStatement::comments`]. Requires
/// `collect_tokens: true` in [`super::ParserConfig`].
#[derive(Debug, Clone, Copy)]
pub struct Comment<'a> {
    /// The full comment text, including delimiters.
    pub text: &'a str,
    /// Whether this is a line (`--`) or block (`/* */`) comment.
    pub kind: CommentKind,
}

pub use crate::grammar::ParserTokenFlags;

/// Token captured from a parsed statement, typed by grammar `G`.
///
/// Returned by [`super::TypedParsedStatement::tokens`]. Requires
/// `collect_tokens: true` in [`super::ParserConfig`].
#[derive(Debug, Clone, Copy)]
pub struct TypedParserToken<'a, G: TypedGrammar> {
    pub(super) text: &'a str,
    pub(super) token_type: G::Token,
    pub(super) flags: ParserTokenFlags,
}

impl<'a, G: TypedGrammar> TypedParserToken<'a, G> {
    /// The source text slice covered by this token.
    pub fn text(&self) -> &'a str {
        self.text
    }

    /// Grammar-typed token variant.
    pub fn token_type(&self) -> G::Token {
        self.token_type
    }

    /// Semantic usage flags inferred by the parser.
    pub fn flags(&self) -> ParserTokenFlags {
        self.flags
    }
}

/// Parser-token alias for grammar-independent pipelines.
pub type AnyParserToken<'a> = TypedParserToken<'a, crate::grammar::AnyGrammar>;

/// Byte range of a macro call that contributed to this parse.
///
/// Returned by [`super::AnyParsedStatement::macro_regions`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacroRegion {
    /// Byte offset of the macro call in the original source.
    pub call_offset: u32,
    /// Byte length of the entire macro call.
    pub call_length: u32,
}

/// Parser's best guess about what kind of token fits next.
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
    /// Convert from a numeric completion-context code.
    ///
    /// Mostly useful for FFI and serialization boundaries.
    pub fn from_raw(v: u32) -> Self {
        match v {
            1 => Self::Expression,
            2 => Self::TableRef,
            _ => Self::Unknown,
        }
    }

    /// Return the numeric completion-context code.
    ///
    /// Mostly useful for FFI and serialization boundaries.
    pub fn raw(self) -> u32 {
        self as u32
    }
}

impl From<CompletionContext> for u32 {
    fn from(v: CompletionContext) -> u32 {
        v.raw()
    }
}

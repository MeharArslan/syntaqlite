// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![cfg_attr(test, expect(clippy::unwrap_used, clippy::similar_names))]

//! Tokenizer and parser for `SQLite` SQL.
//!
//! This crate wraps `SQLite`'s own tokenizer and grammar rules behind
//! safe, zero-dependency Rust APIs. Four design principles guide the library:
//!
//! - **Reliability** — uses `SQLite`'s own tokenizer and grammar rules directly; verified by tests to be identical to `SQLite`'s interpretation.
//! - **Speed** — [`Tokenizer`] is zero-copy; [`Parser`] is minimal-copy, uses arena allocation and can be reused across multiple SQL inputs.
//! - **Portability** — no runtime dependencies in Rust or C beyond the standard library.
//! - **Flexibility** — the grammar system supports database engines which extend `SQLite`'s grammar with their own tokens and rules.
//!
//! # Tokenizing
//!
//! Use [`Tokenizer`] to break SQL source text into [`Token`]s:
//!
//! ```rust
//! let tokenizer = syntaqlite_syntax::Tokenizer::new();
//! for token in tokenizer.tokenize("SELECT 1") {
//!     println!("{:?}: {:?}", token.token_type(), token.text());
//! }
//! ```
//!
//! # Parsing
//!
//! Use [`Parser`] to parse SQL source text into a typed AST:
//!
//! ```rust
//! use syntaqlite_syntax::ParseErrorKind;
//!
//! let parser = syntaqlite_syntax::Parser::new();
//! let mut session = parser.parse("SELECT 1");
//! loop {
//!     match session.next() {
//!         syntaqlite_syntax::ParseOutcome::Ok(statement) => println!("{:?}", statement.root()),
//!         syntaqlite_syntax::ParseOutcome::Err(error) => {
//!             eprintln!("parse error: {}", error.message());
//!             if error.kind() == ParseErrorKind::Fatal {
//!                 break;
//!             }
//!         }
//!         syntaqlite_syntax::ParseOutcome::Done => break,
//!     }
//! }
//! ```
//!
//! # Incremental Parsing
//!
//! Use [`IncrementalParseSession`] when SQL arrives token-by-token
//! (for example in editors and completion engines):
//!
//! ```rust
//! use syntaqlite_syntax::{Parser, TokenType};
//!
//! let parser = Parser::new();
//! let mut session = parser.incremental_parse("SELECT 1");
//!
//! assert!(session.feed_token(TokenType::Select, 0..6).is_none());
//! assert!(session.feed_token(TokenType::Integer, 7..8).is_none());
//!
//! let stmt = session.finish().and_then(Result::ok).unwrap();
//! let _ = stmt.root();
//! ```
//!
//! # Features
//!
//! - `sqlite` *(default)*: enables the built-in `SQLite` grammar
//!   ([`Tokenizer`], [`Token`], and `sqlite::grammar`/`sqlite::ast`).
//! - `serde`: implements [`serde::Serialize`] on [`any::AnyNode`], producing
//!   JSON that mirrors the text dump format.
//! - `serde-json`: adds [`typed::TypedParsedStatement::dump_json`], a
//!   convenience wrapper that calls `serde_json::to_string` on the root node.
//!
//! # Choosing an API Layer
//!
//! - Use top-level [`Parser`] and [`Tokenizer`] for normal `SQLite` application code.
//! - Use [`typed`] when building reusable code over known generated grammars.
//! - Use [`any`] when grammar choice happens at runtime or crosses FFI/plugin boundaries.

// ==== Public API ====

// Top level parser types.
#[doc(inline)]
pub use parser::ParserConfig;
#[cfg(feature = "sqlite")]
#[doc(inline)]
pub use parser::{
    ParseError, ParseErrorKind, ParseOutcome, ParseSession, ParsedStatement, Parser, ParserToken,
};

// Token/comment data types shared across grammars.
#[doc(inline)]
pub use parser::{Comment, CommentKind, CompletionContext, ParserTokenFlags};

// Top-level tokenizer types.
// TokenType is always available — ordinals are stable across all dialects.
#[doc(inline)]
pub use sqlite::tokens::TokenType;
#[cfg(feature = "sqlite")]
#[doc(inline)]
pub use tokenizer::{Token, Tokenizer};

/// Cross-cutting utilities for grammar configuration and compatibility.
///
/// Reach for this module when you need to pin parser behavior to a target
/// `SQLite` release or inspect feature-flag state.
pub mod util;

/// Type-erased variants of every parser and tokenizer type.
///
/// **Most code should not need this module.** If you are working with the
/// `SQLite` grammar — which is the common case — use the top-level
/// [`Parser`], [`ParseSession`], [`Tokenizer`], and [`Token`] types instead.
///
/// ## When to use `any`
///
/// Reach for this module only when you need to work with grammars generically,
/// without knowing the grammar at compile time. The primary use cases are:
///
/// - **Multi-grammar infrastructure** — tools that accept an [`any::AnyGrammar`]
///   from the caller and operate on whichever grammar is handed in (e.g. a
///   generic formatter, a language-server host, or a test harness that runs
///   against several grammars).
/// - **Storage without a lifetime parameter** — [`any::AnyNodeId`] identifies a
///   node in a parse arena and can be stored freely; typed node references
///   borrow the arena and cannot outlive it.
/// - **FFI and plugin boundaries** — [`any::AnyGrammar`] is `Copy + Send + Sync`
///   and is the natural unit to pass across crate boundaries or plugin APIs.
///
/// ## Caveats
///
/// The `Any*` types erase the grammar's token and node enums, replacing them
/// with raw `u32` ordinals. You lose exhaustive `match` on token kinds and
/// the typed accessor methods on AST nodes. Prefer the typed API whenever the
/// grammar is known statically.
pub mod any {
    #[doc(inline)]
    pub use crate::ast::{AnyNode, AnyNodeId, AnyNodeTag, AnyTokenType, FieldValue, NodeFields};
    #[doc(inline)]
    pub use crate::grammar::{AnyGrammar, FieldKind, FieldMeta, KeywordEntry, TokenCategory};
    #[doc(inline)]
    pub use crate::parser::{
        AnyIncrementalParseSession, AnyParseError, AnyParseSession, AnyParsedStatement, AnyParser,
        AnyParserToken, MacroRegion, ParseOutcome,
    };
    #[doc(inline)]
    pub use crate::tokenizer::{AnyToken, AnyTokenizer};
}

/// Generic, grammar-parameterized variants of parser/tokenizer types.
///
/// **Most code should not need this module.** Application code working with
/// the `SQLite` grammar should use the top-level [`Parser`], [`ParseSession`],
/// [`Tokenizer`], and [`Token`] types, which are thin wrappers over the typed
/// internals already instantiated for `SQLite`.
///
/// ## When to use `typed`
///
/// In practice, you will rarely import from this module directly. Its contents
/// are primarily consumed by the grammar generator: the traits
/// [`GrammarNodeType`](typed::GrammarNodeType),
/// [`GrammarTokenType`](typed::GrammarTokenType), and
/// [`TypedNodeId`](typed::TypedNodeId) are implemented automatically for each
/// grammar's generated node and token enums. The generator produces correct
/// implementations — you do not implement or import these manually.
///
/// If you need to write code that works across grammars without grammar-specific
/// types, use [`any`] instead, which provides type-erased equivalents that are
/// far easier to work with.
pub mod typed {
    #[doc(inline)]
    pub use crate::ast::{GrammarNodeType, GrammarTokenType, TypedNodeId, TypedNodeList};
    #[doc(inline)]
    pub use crate::grammar::TypedGrammar;
    #[doc(inline)]
    pub use crate::parser::{
        ParseOutcome, TypedIncrementalParseSession, TypedParseError, TypedParseSession,
        TypedParsedStatement, TypedParser, TypedParserToken,
    };
    #[doc(inline)]
    pub use crate::tokenizer::{TypedToken, TypedTokenizer};

    // Only exposed for use in generated code, not public API.
    #[doc(hidden)]
    pub use crate::grammar::ffi::CGrammar;

    /// Top-level grammar handle for the `SQLite` grammar.
    ///
    /// Most code should not need to call `grammar()` directly; the top-level [`crate::Parser`]
    /// and [`crate::Tokenizer`] types construct it internally. However, if you need to
    /// work with the grammar directly — for example, to inspect its token and node
    /// metadata, or to construct a parser or tokenizer manually — you can obtain a
    /// handle with `grammar()`.
    /// Top-level typed grammar handle type for the `SQLite` grammar.
    ///
    /// Useful when you need to name the type parameter in `TypedDialect<Grammar>`.
    #[cfg(feature = "sqlite")]
    pub use crate::sqlite::grammar::Grammar;
    #[cfg(feature = "sqlite")]
    pub use crate::sqlite::grammar::grammar;
}

/// Generated typed AST for the built-in `SQLite` grammar.
///
/// Re-exports every generated node struct, enum, and accessor type for the
/// `SQLite` grammar. Import from here when you need to name concrete node
/// types — for example, when pattern-matching on a [`ParsedStatement`] or
/// traversing the parse tree.
#[cfg(feature = "sqlite")]
pub mod nodes {
    pub use crate::sqlite::ast::*;
}

// Top-level incremental parse session type (SQLite grammar).
#[cfg(feature = "sqlite")]
#[doc(inline)]
pub use parser::IncrementalParseSession;

// ==== Internal modules ====

pub(crate) mod ast;
mod grammar;
pub(crate) mod parser;
pub(crate) mod tokenizer;

// `sqlite` module is always present; individual sub-modules are gated inside it.
pub(crate) mod sqlite;

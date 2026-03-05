// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::similar_names))]

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
//! while let Some(statement) = session.next() {
//!     match statement {
//!         Ok(statement) => println!("{:?}", statement.root()),
//!         Err(error) => {
//!             eprintln!("parse error: {}", error.message());
//!             if error.kind() == ParseErrorKind::Fatal {
//!                 break;
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! # Features
//!
//! - `sqlite` *(default)*: enables the built-in `SQLite` dialect
//!   ([`Tokenizer`], [`Token`], and `sqlite::grammar`/`sqlite::ast`).

// ==== Public API ====

// Top level parser types.
#[doc(inline)]
pub use parser::ParserConfig;
#[cfg(feature = "sqlite")]
#[doc(inline)]
pub use parser::{ParseError, ParseErrorKind, ParseSession, ParsedStatement, Parser, ParserToken};

// Token/comment data types shared across dialects.
#[doc(inline)]
pub use parser::{Comment, CommentKind, CompletionContext, ParserTokenFlags};

// Top-level tokenizer types.
#[cfg(feature = "sqlite")]
#[doc(inline)]
pub use sqlite::tokens::TokenType;
#[cfg(feature = "sqlite")]
#[doc(inline)]
pub use tokenizer::{Token, Tokenizer};

/// AST accessor traits implemented by generated dialect types.
#[doc(hidden)]
pub mod ast_traits;

/// Shared utilities (e.g. [`SqliteVersion`](util::SqliteVersion)).
pub mod util;

/// Type-erased variants of every parser and tokenizer type.
///
/// **Most code should not need this module.** If you are working with the
/// `SQLite` dialect — which is the common case — use the top-level
/// [`Parser`], [`ParseSession`], [`Tokenizer`], and [`Token`] types instead.
///
/// ## When to use `any`
///
/// Reach for this module only when you need to work with grammars generically,
/// without knowing the dialect at compile time. The primary use cases are:
///
/// - **Multi-dialect infrastructure** — tools that accept an [`any::AnyGrammar`]
///   from the caller and operate on whichever dialect is handed in (e.g. a
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
/// The `Any*` types erase the dialect's token and node enums, replacing them
/// with raw `u32` ordinals. You lose exhaustive `match` on token kinds and
/// the typed accessor methods on AST nodes. Prefer the typed API whenever the
/// dialect is known statically.
pub mod any {
    #[doc(inline)]
    pub use crate::ast::{AnyNode, AnyNodeId, AnyNodeTag, AnyTokenType, FieldValue, NodeFields};
    #[doc(inline)]
    pub use crate::grammar::{AnyGrammar, FieldKind, FieldMeta, KeywordEntry, TokenCategory};
    #[doc(inline)]
    pub use crate::parser::{
        AnyIncrementalParseSession, AnyParseError, AnyParseSession, AnyParsedStatement, AnyParser,
        AnyParserToken, MacroRegion,
    };
    #[doc(inline)]
    pub use crate::tokenizer::{AnyToken, AnyTokenizer};
}

/// Generic, grammar-parameterized variants of every parser and tokenizer type.
///
/// **Most code should not need this module.** Application code working with
/// the `SQLite` dialect should use the top-level [`Parser`], [`ParseSession`],
/// [`Tokenizer`], and [`Token`] types, which are thin wrappers over the typed
/// internals already instantiated for `SQLite`.
///
/// ## When to use `typed`
///
/// In practice, you will rarely import from this module directly. Its contents
/// are primarily consumed by the dialect generator: the traits
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
        TypedIncrementalParseSession, TypedParseError, TypedParseSession, TypedParsedStatement,
        TypedParser, TypedParserToken,
    };
    #[doc(inline)]
    pub use crate::tokenizer::{TypedToken, TypedTokenizer};

    // Only exposed for use in generated code, not public API.
    #[doc(hidden)]
    pub use crate::grammar::ffi::CGrammar;

    /// Top-level grammar handle for the `SQLite` dialect.
    ///
    /// Most code should not need to call `grammar()` directly; the top-level [`crate::Parser`]
    /// and [`crate::Tokenizer`] types construct it internally. However, if you need to
    /// work with the grammar directly — for example, to inspect its token and node
    /// metadata, or to construct a parser or tokenizer manually — you can obtain a
    /// handle with `grammar()`.
    #[cfg(feature = "sqlite")]
    pub use crate::sqlite::grammar::grammar;
}

/// Typed AST node types for the built-in `SQLite` dialect.
///
/// Re-exports every generated node struct, enum, and accessor type for the
/// `SQLite` dialect. Import from here when you need to name concrete node
/// types — for example, when pattern-matching on a [`ParsedStatement`] or
/// traversing the parse tree.
#[cfg(feature = "sqlite")]
pub mod nodes {
    pub use crate::sqlite::ast::*;
}

// Top-level incremental parse session type (SQLite dialect).
#[cfg(feature = "sqlite")]
#[doc(inline)]
pub use parser::IncrementalParseSession;

// ==== Internal modules ====

pub(crate) mod ast;
pub(crate) mod cflags;
mod grammar;
pub(crate) mod parser;
pub(crate) mod tokenizer;

#[cfg(feature = "sqlite")]
pub(crate) mod sqlite;

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use std::ffi::CString;
    use std::panic::{self, AssertUnwindSafe};

    use super::{CommentKind, ParseErrorKind, Parser, ParserConfig, TokenType, Tokenizer};

    #[test]
    fn tokenizer_emits_expected_core_tokens() {
        let tokenizer = Tokenizer::new();
        let tokens: Vec<_> = tokenizer
            .tokenize("SELECT x, 1 FROM t;")
            .filter(|token| !matches!(token.token_type(), TokenType::Space | TokenType::Comment))
            .map(|token| (token.token_type(), token.text().to_owned()))
            .collect();

        assert_eq!(
            tokens,
            vec![
                (TokenType::Select, "SELECT".to_owned()),
                (TokenType::Id, "x".to_owned()),
                (TokenType::Comma, ",".to_owned()),
                (TokenType::Integer, "1".to_owned()),
                (TokenType::From, "FROM".to_owned()),
                (TokenType::Id, "t".to_owned()),
                (TokenType::Semi, ";".to_owned()),
            ]
        );
    }

    #[test]
    fn tokenizer_cstr_matches_str_path() {
        let source = CString::new("SELECT 1;").expect("source has no interior NUL");
        let tokenizer = Tokenizer::new();

        let from_str: Vec<_> = tokenizer
            .tokenize(source.to_str().expect("source is UTF-8"))
            .map(|token| (token.token_type(), token.text().to_owned()))
            .collect();

        let from_cstr: Vec<_> = tokenizer
            .tokenize_cstr(source.as_c_str())
            .map(|token| (token.token_type(), token.text().to_owned()))
            .collect();

        assert_eq!(from_str, from_cstr);
    }

    #[test]
    fn tokenizer_allows_only_one_live_cursor() {
        let tokenizer = Tokenizer::new();
        let mut cursor = tokenizer.tokenize("SELECT 1;");
        assert!(cursor.next().is_some());

        let reentrant_attempt = panic::catch_unwind(AssertUnwindSafe(|| {
            let _cursor = tokenizer.tokenize("SELECT 2;");
        }));
        assert!(reentrant_attempt.is_err());

        drop(cursor);
        let second_count = tokenizer.tokenize("SELECT 2;").count();
        assert!(second_count > 0);
    }

    #[test]
    fn parser_continues_after_statement_error() {
        let parser = Parser::new();
        let mut session = parser.parse("SELECT 1; SELECT ; SELECT 2;");

        let first = session.next().expect("first statement is present");
        assert!(matches!(first, Ok(statement) if statement.root().is_some()));

        let second = session.next().expect("second statement is present");
        let error = match second {
            Ok(_) => panic!("second statement should fail"),
            Err(error) => error,
        };
        assert!(!error.message().is_empty());
        assert_ne!(error.is_fatal(), error.is_recovered());
        assert!(matches!(
            error.kind(),
            ParseErrorKind::Recovered | ParseErrorKind::Fatal
        ));

        let third = session.next().expect("third statement is present");
        assert!(matches!(third, Ok(statement) if statement.root().is_some()));
        assert!(session.next().is_none());
    }

    #[test]
    fn parser_collect_tokens_and_comments() {
        let parser = Parser::with_config(&ParserConfig::default().with_collect_tokens(true));
        let mut session = parser.parse("/* lead */ SELECT 1 -- tail\n;");

        let statement = session
            .next()
            .expect("statement is present")
            .expect("statement parses successfully");

        let token_types: Vec<_> = statement.tokens().map(|token| token.token_type()).collect();
        assert!(token_types.contains(&TokenType::Select));
        assert!(token_types.contains(&TokenType::Integer));

        let comments: Vec<_> = statement.comments().collect();
        assert!(
            comments
                .iter()
                .any(|comment| comment.kind == CommentKind::Block && comment.text.contains("lead"))
        );
        assert!(
            comments
                .iter()
                .any(|comment| comment.kind == CommentKind::Line && comment.text.contains("tail"))
        );
    }

    #[test]
    fn parser_allows_only_one_live_session() {
        let parser = Parser::new();
        let session = parser.parse("SELECT 1;");

        let reentrant_attempt = panic::catch_unwind(AssertUnwindSafe(|| {
            let _session = parser.parse("SELECT 2;");
        }));
        assert!(reentrant_attempt.is_err());

        drop(session);

        let mut second = parser.parse("SELECT 2;");
        let result = second.next().expect("statement is present");
        assert!(matches!(result, Ok(statement) if statement.root().is_some()));
    }
}

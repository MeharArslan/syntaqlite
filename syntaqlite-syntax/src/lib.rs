// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::similar_names))]

//! Low-level tokenizer and parser for `SQLite` SQL.
//!
//! `syntaqlite-syntax` wraps `SQLite`'s own tokenizer and grammar rules — extracted
//! directly from `SQLite`'s source and verified by tests to match exactly — behind
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
//! ```rust,ignore
//! let grammar = syntaqlite_syntax::sqlite::grammar::grammar();
//! let tokenizer = syntaqlite_syntax::Tokenizer::new(grammar);
//! for token in tokenizer.tokenize("SELECT 1") {
//!     println!("{:?}: {:?}", token.token_type(), token.text());
//! }
//! ```
//!
//! # Parsing
//!
//! Use [`Parser`] to parse SQL source text into a typed AST:
//!
//! ```rust,ignore
//! let grammar = syntaqlite_syntax::sqlite::grammar::grammar();
//! let parser = syntaqlite_syntax::Parser::new(grammar.into_raw());
//! let cursor = parser.parse("SELECT 1");
//! while let Some(stmt) = cursor.next_statement() {
//!     println!("{stmt:?}");
//! }
//! ```
//!
//! # Features
//!
//! - `sqlite` *(default)*: enables the built-in `SQLite` dialect
//!   ([`Tokenizer`], [`Token`], and `sqlite::grammar`/`sqlite::ast`).

// ==== Public API ====

// Top level parser types.
#[cfg(feature = "sqlite")]
#[doc(inline)]
pub use parser::{ParseError, ParseSession, Parser, StatementResult};

// Top-level tokenizer types.
#[cfg(feature = "sqlite")]
#[doc(inline)]
pub use tokenizer::{Token, Tokenizer};

/// AST accessor traits implemented by generated dialect types.
pub mod ast_traits;

/// Tokenizer for `SQLite` SQL text.
pub mod tokenizer;

/// Shared utilities (e.g. [`SqliteVersion`](util::SqliteVersion)).
pub mod util;

/// Grammar-agnostic AST node types and traits.
pub mod ast;

/// Incremental parse session types.
pub mod incremental;

/// Built-in `SQLite` dialect: AST types, token types, and grammar handle.
///
/// Use `sqlite::grammar::grammar()` to obtain a grammar handle and pass it to
/// [`Tokenizer`] or [`Parser`].
#[cfg(feature = "sqlite")]
pub mod sqlite;

// ==== Internal modules ====

mod cflags;
mod grammar;
/// Low-level typed parser and parse session types.
pub mod parser;

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::similar_names))]

//! Low-level tokenizer and parser for SQLite SQL.
//!
//! `syntaqlite-syntax` wraps SQLite's own tokenizer and grammar rules — extracted
//! directly from SQLite's source and verified by tests to match exactly — behind
//! safe, zero-dependency Rust APIs. Four design principles guide the library
//! (expanded on in the [Design principles](#design-principles) section below):
//!
//! - **Reliability** — uses SQLite's own tokenizer and grammar rules directly; verified by tests to be identical to SQLite's interpretation.
//! - **Speed** — [`Tokenizer`] is zero-copy; [`Parser`] is minimal-copy and uses arena allocation allowing reuse across multiple SQL inputs.
//! - **Portability** — no runtime dependencies in Rust or C beyond the standard library.
//! - **Flexibility** — the grammar system supports database engines which extend SQLite's grammar with their own tokens and rules.
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
//! # Design principles
//!
//! ## Reliability
//!
//! The tokenizer is extracted directly from SQLite's source code, and the grammar
//! rules are verified by tests to be identical to SQLite's own Lemon grammar.
//! The library tokenizes and parses SQL exactly as SQLite does.
//!
//! ## Speed
//!
//! [`Token::text`] is a zero-copy slice into the original source string.
//! [`Parser`] reuses its internal arena across calls, so repeated parses avoid
//! repeated allocation. Typed AST view structs are zero-sized arena pointers.
//!
//! ## Portability
//!
//! `syntaqlite-syntax`, like all code under the syntaqlite umbrella, has no
//! runtime Rust dependencies and no C dependencies beyond the standard library.
//! The only build-time dependency is [`cc`](https://crates.io/crates/cc).
//!
//! ## Flexibility
//!
//! Database engines which extend SQLite with additional syntax can provide their
//! own grammar and reuse the same tokenization and parsing infrastructure.
//!
//! # Features
//!
//! - `sqlite` *(default)*: enables the built-in SQLite dialect
//!   ([`Tokenizer`], [`Token`], [`TokenCursor`], and `sqlite::grammar`/`sqlite::ast`).

// ==== Public API ====

// Top level parser types.
#[cfg(feature = "sqlite")]
pub use parser::{Parser, StatementCursor, ParseError};

// Top-level tokenizer types.
#[cfg(feature = "sqlite")]
pub use tokenizer::{Token, TokenCursor, Tokenizer};

// TODO(claude): document this.
pub mod tokenizer;

// TODO(claude): document this.
pub mod ast_traits;

// ==== Internal modules ====

mod ast;
mod cflags;
mod grammar;
mod incremental;
mod parser;
mod util;

#[cfg(feature = "sqlite")]
mod sqlite;

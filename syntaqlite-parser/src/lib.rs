// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![warn(unreachable_pub)]

//! Raw FFI bindings and dialect-agnostic arena infrastructure for the
//! syntaqlite C parser engine.
//!
//! This crate compiles the C parser/tokenizer (via `build.rs`) and exports
//! the Rust-side `#[repr(C)]` mirror types, `extern "C"` declarations,
//! and the grammar-agnostic arena / session machinery.

pub mod ast_traits;

// ── Dialect ──────────────────────────────────────────────────────────────────

pub use crate::dialect::Dialect;
pub use crate::dialect::{DialectEnv, FfiDialect, SchemaContribution, SchemaKind};
// TODO(lalitm): FieldMeta should be deleted entirely; callers should use the
// safe field accessors on Dialect instead of reading C metadata structs directly.
pub use crate::dialect::FieldMeta;
pub use crate::dialect::{FIELD_BOOL, FIELD_ENUM, FIELD_NODE_ID, FIELD_SPAN};
pub use crate::dialect_traits::{DialectNodeType, DialectTokenType, NodeFamily, NodeId};

// ── Core node/arena types ─────────────────────────────────────────────────────

pub use crate::nodes::{ArenaNode, FieldVal, Fields, NodeList, RawNodeId, SourceSpan};
pub use crate::session::{ErrorSpan, NodeRef, ParseError, RawParseResult};
pub use crate::typed_list::TypedList;

// ── C parser FFI types ────────────────────────────────────────────────────────

pub use crate::parser::{
    Comment, CommentKind, ErrorNode, MacroRegion, MemMethods, ParseResult, Parser,
    SYNTAQLITE_ERROR_NODE_TAG, TOKEN_FLAG_AS_FUNCTION, TOKEN_FLAG_AS_ID, TOKEN_FLAG_AS_TYPE, Token,
    TokenPos, Tokenizer,
};

// ── Raw (grammar-agnostic) parsers and tokenizer ──────────────────────────────

pub use crate::raw_incremental::{RawIncrementalCursor, RawIncrementalParser};
pub use crate::raw_session::{ParserConfig, RawParser, RawStatementCursor};
pub use crate::raw_tokenizer::{RawToken, RawTokenCursor, RawTokenizer};

// ── Function availability catalog ─────────────────────────────────────────────

pub use crate::catalog::{FunctionCategory, FunctionInfo, is_function_available};
pub use crate::cflag_versions::available_functions;
pub use crate::cflag_versions::{cflag_table, parse_cflag_name, parse_sqlite_version};
pub use crate::dialect::ffi::{CflagInfo, Cflags};

pub(crate) mod catalog;
pub(crate) mod cflag_versions;
pub(crate) mod dialect;
pub(crate) mod dialect_traits;
pub(crate) mod functions_catalog;
pub(crate) mod nodes;
pub(crate) mod parser;
pub(crate) mod raw_incremental;
pub(crate) mod raw_session;
pub(crate) mod raw_tokenizer;
pub(crate) mod session;
pub(crate) mod typed_list;

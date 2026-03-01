// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Grammar-agnostic parser infrastructure.
//!
//! This module contains the core parsing machinery shared by all dialects:
//! tokenization, incremental token feeding, session management, and the
//! arena-allocated node representation. Most users should use the typed
//! wrappers ([`crate::Parser`], [`crate::ast`]) rather than these
//! internals directly.

pub(crate) mod ffi;
pub(crate) mod nodes;
pub(crate) mod session;
pub(crate) mod token_parser;
pub(crate) mod tokenizer;
pub(crate) mod typed_list;

pub mod typed;

// ── Crate-internal convenience re-exports ────────────────────────────────
//
// These keep internal `use crate::parser::Foo` paths working without
// exposing the types as `syntaqlite::parser::Foo` to downstream users.

pub(crate) use ffi::{Comment, CommentKind};
pub(crate) use nodes::{ArenaNode, FieldVal, Fields, NodeId, SourceSpan};
pub(crate) use session::{RawParser, RawNodeReader, NodeRef, ParseError};
pub(crate) use token_parser::RawIncrementalParser;
pub(crate) use tokenizer::RawTokenizer;
pub(crate) use typed_list::{FromArena, TypedList};

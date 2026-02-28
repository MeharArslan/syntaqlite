// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

pub(crate) mod ffi;
#[doc(hidden)]
pub mod nodes;
mod session;
mod token_parser;
mod tokenizer;
mod typed_list;

pub use ffi::{
    Comment, CommentKind, MacroRegion, TOKEN_FLAG_AS_FUNCTION, TOKEN_FLAG_AS_ID,
    TOKEN_FLAG_AS_TYPE, TokenPos,
};
pub use nodes::{ArenaNode, FieldVal, Fields, NodeId, NodeList, SourceSpan};
pub use session::{
    CursorBase, ErrorSpan, NodeReader, ParseError, Parser, ParserConfig, StatementCursor,
};
pub use token_parser::{LowLevelCursor, LowLevelParser};
pub use tokenizer::{RawToken, TokenCursor, Tokenizer};
pub use typed_list::{FromArena, TypedList};

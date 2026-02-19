mod generated;
mod dialect;
mod fmt_data;
pub mod nodes;
mod tokenizer;

mod parser;

// Re-export runtime types that callers need (but NOT Parser or Dialect).
pub use syntaqlite_runtime::{
    FieldVal, Fields, MacroRegion, NodeId, NodeList, ParseError,
    Session, SourceSpan, Trivia, TriviaKind,
};

// SQLite-specific parser (wraps runtime Parser with the SQLite dialect).
pub use parser::Parser;

// Dialect-specific exports
pub use generated::nodes::*;
pub use generated::tokens::TokenType;
pub use dialect::{dump_node, SessionExt, NODE_INFO};
pub use tokenizer::{Token, TokenStream, Tokenizer};

// Formatter re-exports (runtime engine + dialect data)
pub use syntaqlite_runtime::fmt::{
    DocArena, DocId, NIL_DOC, FormatConfig, KeywordCase,
    FmtCtx, NodeFmt, NodeInfo, TriviaCtx,
    format_node, format_node_with_trivia, first_source_offset, last_source_offset,
    render,
};
pub use fmt_data::{dispatch, ctx};

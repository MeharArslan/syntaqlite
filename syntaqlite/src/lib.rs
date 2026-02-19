mod generated;
mod dialect;
mod fmt_data;
pub(crate) mod nodes;
mod parser;

// ── Root: the essentials ────────────────────────────────────────────────
pub use parser::Parser;
pub use syntaqlite_runtime::{NodeId, ParseError, Session, SourceSpan};

// ── AST types & inspection ──────────────────────────────────────────────
pub mod ast {
    pub use crate::generated::nodes::*;
    pub use crate::dialect::{dump_node, SessionExt, NODE_INFO};
    pub use syntaqlite_runtime::{
        MacroRegion, NodeList, Trivia, TriviaKind,
    };
}

// ── Formatter ───────────────────────────────────────────────────────────
pub mod fmt {
    pub use syntaqlite_runtime::fmt::{
        DocArena, FormatConfig, KeywordCase, TriviaCtx,
        format_node, format_node_with_trivia, first_source_offset, render,
    };
    pub use crate::fmt_data::{dispatch, ctx};
    pub use crate::ast::NODE_INFO;
}

// ── Tokenizer ───────────────────────────────────────────────────────────
pub mod tokenizer {
    mod inner {
        pub use crate::tokenizer_impl::*;
    }
    pub use inner::*;
    pub use crate::generated::tokens::TokenType;
}

// Private implementation module for the tokenizer wrapper types.
#[path = "tokenizer.rs"]
mod tokenizer_impl;

mod generated;
#[cfg(feature = "parser")]
mod dialect;
#[cfg(feature = "fmt")]
mod fmt_data;
#[cfg(feature = "parser")]
pub(crate) mod nodes;
#[cfg(feature = "parser")]
mod parser;

// ── Root: the essentials ────────────────────────────────────────────────
#[cfg(feature = "parser")]
pub use dialect::sqlite_dialect;
#[cfg(feature = "parser")]
pub use parser::Parser;
#[cfg(feature = "parser")]
pub use syntaqlite_runtime::{NodeId, ParseError, Session, SourceSpan};

// ── AST types & inspection ──────────────────────────────────────────────
#[cfg(feature = "parser")]
pub mod ast {
    pub use crate::generated::nodes::*;
    pub use crate::dialect::{dump_node, SessionExt};
    pub use syntaqlite_runtime::{
        MacroRegion, NodeList, Trivia, TriviaKind,
    };
}

// ── Formatter ───────────────────────────────────────────────────────────
#[cfg(feature = "fmt")]
pub mod fmt {
    pub use syntaqlite_runtime::fmt::{
        DocArena, FormatConfig, KeywordCase, TriviaCtx,
        format_node, format_node_with_trivia, first_source_offset, render,
    };
    pub use crate::fmt_data::{dispatch, ctx, NODE_INFO};
}

// ── Tokenizer ───────────────────────────────────────────────────────────
#[cfg(feature = "parser")]
pub mod tokenizer {
    mod inner {
        pub use crate::tokenizer_impl::*;
    }
    pub use inner::*;
    pub use crate::generated::tokens::TokenType;
}

// Private implementation module for the tokenizer wrapper types.
#[cfg(feature = "parser")]
#[path = "tokenizer.rs"]
mod tokenizer_impl;

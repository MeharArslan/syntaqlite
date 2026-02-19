#[cfg(feature = "parser")]
pub mod parser;

#[cfg(feature = "parser")]
pub use parser::{
    dump_node_with, Dialect, FieldDescriptor, FieldKind, FieldVal, Fields, NodeId, NodeList,
    SourceSpan, MacroRegion, ParseError, Parser, Session, Trivia, TriviaKind,
};

#[cfg(feature = "fmt")]
pub mod fmt;

#[cfg(feature = "parser")]
mod dialect;

#[cfg(feature = "parser")]
pub use dialect::{DialectInfo, DialectTypes, SessionExt};

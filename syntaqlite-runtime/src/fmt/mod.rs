pub mod bytecode;
pub mod bytecode_format;
mod config;
pub mod doc;
mod format;
mod formatter;
pub mod interpret;
pub mod ops;
pub mod render;
pub mod trivia;

// ── Primary public API ─────────────────────────────────────────────────
pub use config::{FormatConfig, KeywordCase};
pub use formatter::Formatter;

// ── Low-level types (for internal tests and codegen) ───────────────────
pub use doc::{DocArena, DocId, NIL_DOC};
pub use interpret::FmtCtx;
pub use render::render;

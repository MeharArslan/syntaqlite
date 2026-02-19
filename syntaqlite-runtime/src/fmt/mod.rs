mod bytecode;
pub mod bytecode_format;
mod config;
mod doc;
mod format;
mod formatter;
mod interpret;
mod ops;
mod render;
mod trivia;

// ── Primary public API ─────────────────────────────────────────────────
pub use config::{FormatConfig, KeywordCase};
pub use formatter::Formatter;

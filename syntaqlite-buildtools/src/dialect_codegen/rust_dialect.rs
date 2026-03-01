// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use crate::util::rust_writer::RustWriter;

const LIB_MODULE_DECLS: &str = r#"
mod ffi;
/// Typed AST nodes for this dialect.
///
/// Each SQL statement type (e.g. `SELECT`, `INSERT`) has a corresponding struct
/// with typed accessors for its fields. The top-level enum is [`ast::Stmt`],
/// returned by [`StatementCursor::next_statement`] and
/// [`LowLevelCursor::finish`](low_level::LowLevelCursor::finish).
pub mod ast;
mod wrappers;
"#;

const LIB_LOW_LEVEL_MOD: &str = r#"
/// Low-level APIs for advanced use cases (e.g. custom token feeding/tokenizing).
pub mod low_level {
    pub use crate::wrappers::{LowLevelCursor, LowLevelParser, Tokenizer, TokenCursor};
    pub use crate::tokens::TokenType;

    /// Access the dialect handle (for use with `syntaqlite` APIs).
    pub fn dialect() -> &'static syntaqlite::Dialect<'static> {
        &crate::DIALECT
    }
}
"#;

const LIB_EXPORTS: &str = r#"
pub use wrappers::{Formatter, Parser, StatementCursor};
pub use syntaqlite::ParseError;
"#;

const LIB_CONFIG_MOD: &str = r#"
/// Configuration types for parsers and formatters.
pub mod config {
    pub use syntaqlite::dialect::ffi::{CflagInfo, Cflags, DialectConfig, cflag_table};
    pub use syntaqlite::fmt::{FormatConfig, KeywordCase};
    pub use syntaqlite::parser::ParserConfig;
}
"#;

const WRAPPERS_PRELUDE: &str = r#"
use std::ops::Range;

use crate::ast::{FromArena, Stmt};
use crate::low_level::TokenType;
use crate::ParseError;
"#;

const WRAPPER_PARSER: &str = r#"
/// A parser pre-configured for this dialect.
///
/// Returns typed `StatementCursor` wrappers from `parse()`.
pub struct Parser {
    inner: syntaqlite::Parser,
}

impl Parser {
    /// Create a new parser with default configuration.
    pub fn new() -> Self {
        Parser { inner: syntaqlite::Parser::with_dialect(&crate::DIALECT) }
    }

    /// Create a parser with the given configuration.
    pub fn with_config(config: &crate::config::ParserConfig) -> Self {
        Parser { inner: syntaqlite::Parser::with_dialect_config(&crate::DIALECT, config) }
    }

    /// Access the current configuration.
    pub fn config(&self) -> &crate::config::ParserConfig {
        self.inner.config()
    }

    /// Parse source text and return a `StatementCursor` for iterating statements.
    pub fn parse<'a>(&'a mut self, source: &'a str) -> StatementCursor<'a> {
        StatementCursor { inner: self.inner.parse(source) }
    }
}
"#;

const WRAPPER_STATEMENT_CURSOR: &str = r#"
/// A high-level parsing cursor with typed node access.
pub struct StatementCursor<'a> {
    inner: syntaqlite::StatementCursor<'a>,
}

impl<'a> StatementCursor<'a> {
    /// Parse and return the next SQL statement as a typed `Stmt`.
    ///
    /// The returned `Stmt` borrows this cursor, so it cannot outlive it.
    /// Returns `None` when all statements have been consumed.
    pub fn next_statement(&mut self) -> Option<Result<Stmt<'_>, ParseError>> {
        let id = match self.inner.next_statement()? {
            Ok(id) => id,
            Err(e) => return Some(Err(e)),
        };
        let reader = self.inner.reader();
        Some(Ok(Stmt::from_arena(reader, id).expect("parser returned invalid node")))
    }
}
"#;

const WRAPPER_LOW_LEVEL_PARSER: &str = r#"
/// A low-level parser for token-by-token feeding.
///
/// Feed tokens one at a time via `LowLevelCursor`.
pub struct LowLevelParser {
    inner: syntaqlite::parser::LowLevelParser,
}

impl LowLevelParser {
    /// Create a new low-level parser with default configuration.
    pub fn new() -> Self {
        LowLevelParser {
            inner: syntaqlite::parser::LowLevelParser::with_dialect(&crate::DIALECT),
        }
    }

    /// Create a low-level parser with the given configuration.
    pub fn with_config(config: &crate::config::ParserConfig) -> Self {
        LowLevelParser {
            inner: syntaqlite::parser::LowLevelParser::with_dialect_config(&crate::DIALECT, config),
        }
    }

    /// Bind source text and return a `LowLevelCursor` for token feeding.
    pub fn feed<'a>(&'a mut self, source: &'a str) -> LowLevelCursor<'a> {
        LowLevelCursor { inner: self.inner.feed(source) }
    }
}
"#;

const WRAPPER_LOW_LEVEL_CURSOR: &str = r#"
/// A low-level cursor for feeding tokens one at a time.
///
/// After calling `finish()`, no further feeding methods may be called.
pub struct LowLevelCursor<'a> {
    inner: syntaqlite::parser::LowLevelCursor<'a>,
}

impl<'a> LowLevelCursor<'a> {
    /// Feed a typed token to the parser.
    ///
    /// Returns `Ok(Some(stmt))` when a statement completes,
    /// `Ok(None)` to keep going, or `Err` on parse error.
    ///
    /// The returned `Stmt` borrows this cursor, so it cannot be held
    /// across further `feed_token` calls.
    ///
    /// `span` is a byte range into the source text bound by this cursor.
    pub fn feed_token(
        &mut self,
        token_type: TokenType,
        span: Range<usize>,
    ) -> Result<Option<Stmt<'_>>, ParseError> {
        match self.inner.feed_token(token_type.into(), span)? {
            None => Ok(None),
            Some(id) => {
                let reader = self.inner.base().reader();
                Ok(Some(Stmt::from_arena(reader, id).expect("parser returned invalid node")))
            }
        }
    }

    /// Signal end of input.
    ///
    /// Returns `Ok(Some(stmt))` if a final statement completed,
    /// `Ok(None)` if there was nothing pending, or `Err` on parse error.
    ///
    /// After calling `finish()`, no further feeding methods may be called.
    pub fn finish(&mut self) -> Result<Option<Stmt<'_>>, ParseError> {
        match self.inner.finish()? {
            None => Ok(None),
            Some(id) => {
                let reader = self.inner.base().reader();
                Ok(Some(Stmt::from_arena(reader, id).expect("parser returned invalid node")))
            }
        }
    }

    /// Mark subsequent fed tokens as being inside a macro expansion.
    pub fn begin_macro(&mut self, call_offset: u32, call_length: u32) {
        self.inner.begin_macro(call_offset, call_length)
    }

    /// End the innermost macro expansion region.
    pub fn end_macro(&mut self) {
        self.inner.end_macro()
    }
}
"#;

const WRAPPER_FORMATTER: &str = r#"
/// SQL formatter pre-configured for this dialect.
pub struct Formatter {
    inner: syntaqlite::fmt::Formatter<'static>,
}

impl Formatter {
    /// Create a formatter with default configuration.
    pub fn new() -> Result<Self, &'static str> {
        let inner = syntaqlite::fmt::Formatter::with_dialect(&crate::DIALECT)?;
        Ok(Formatter { inner })
    }

    /// Create a formatter with the given configuration.
    pub fn with_config(config: crate::config::FormatConfig) -> Result<Self, &'static str> {
        let inner = syntaqlite::fmt::Formatter::with_dialect_config(&crate::DIALECT, config)?;
        Ok(Formatter { inner })
    }

    /// Access the current configuration.
    pub fn config(&self) -> &crate::config::FormatConfig {
        self.inner.config()
    }

    /// Format SQL source text.
    pub fn format(
        &mut self,
        source: &str,
    ) -> Result<String, ParseError> {
        self.inner.format(source)
    }
}
"#;

const WRAPPER_TOKENIZER: &str = r#"
/// A tokenizer for SQL.
pub struct Tokenizer {
    inner: syntaqlite::parser::Tokenizer,
}

impl Tokenizer {
    /// Create a new tokenizer.
    pub fn new() -> Self {
        Tokenizer {
            inner: syntaqlite::parser::Tokenizer::with_dialect(*crate::DIALECT),
        }
    }

    /// Bind source text and return a cursor for iterating typed tokens.
    pub fn tokenize<'a>(&'a mut self, source: &'a str) -> TokenCursor<'a> {
        TokenCursor {
            inner: self.inner.tokenize(source),
        }
    }

    /// Zero-copy variant: bind a null-terminated source and return a
    /// `TokenCursor`. The source must be valid UTF-8 (panics otherwise).
    pub fn tokenize_cstr<'a>(&'a mut self, source: &'a std::ffi::CStr) -> TokenCursor<'a> {
        TokenCursor {
            inner: self.inner.tokenize_cstr(source),
        }
    }
}
"#;

const WRAPPER_TOKEN_CURSOR: &str = r#"
/// An active tokenizer session yielding typed tokens.
pub struct TokenCursor<'a> {
    inner: syntaqlite::parser::TokenCursor<'a>,
}

impl<'a> Iterator for TokenCursor<'a> {
    type Item = (TokenType, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let raw = self.inner.next()?;
        let tt = TokenType::from_raw(raw.token_type)
            .unwrap_or(TokenType::Illegal);
        Some((tt, raw.text))
    }
}
"#;

fn emit_section(w: &mut RustWriter, section: &str) {
    w.lines(section);
    w.newline();
}

pub fn generate_rust_lib(dialect_fn: &str) -> String {
    let mut w = RustWriter::new();
    w.file_header();
    emit_section(&mut w, LIB_MODULE_DECLS);
    w.lines(&format!(
        r#"
use std::sync::LazyLock;

use syntaqlite::dialect::ffi as dialect_ffi;
unsafe extern "C" {{
    fn {dialect_fn}() -> *const dialect_ffi::Dialect;
}}

static DIALECT: LazyLock<syntaqlite::Dialect<'static>> =
    LazyLock::new(|| unsafe {{ syntaqlite::Dialect::from_raw({dialect_fn}()) }});
"#
    ));
    w.newline();
    emit_section(&mut w, LIB_LOW_LEVEL_MOD);
    emit_section(&mut w, LIB_EXPORTS);
    emit_section(&mut w, LIB_CONFIG_MOD);
    w.line("mod tokens;");
    w.finish()
}

/// Generate `build.rs` for a dialect crate.
///
/// The generated build script compiles the dialect's C sources via `cc`
/// and handles version/cflag pinning by passing `-D` flags to the C compiler.
pub fn generate_rust_build_rs(dialect_name: &str) -> String {
    let mut w = RustWriter::new();
    w.file_header();
    w.lines(&format!(
        r#"
use std::env;
use std::path::PathBuf;

fn main() {{
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let csrc = manifest_dir.join("csrc");
    let runtime_include = manifest_dir.join("../syntaqlite-parser-sys/include");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // Dialect sources — Lemon parser, tokenizer, keyword lookup, and dialect glue.
    // Grammar-agnostic engine C is built by the syntaqlite crate.
    let mut build = cc::Build::new();
    build
        .file(csrc.join("dialect.c"))
        .file(csrc.join("{dialect_name}_parse.c"))
        .file(csrc.join("{dialect_name}_tokenize.c"))
        .file(csrc.join("{dialect_name}_keyword.c"))
        .include(&manifest_dir) // for dialect csrc/ headers
        .include(manifest_dir.join("include")) // for dialect include/ headers
        .include(&runtime_include) // for shared syntaqlite/*.h and syntaqlite_ext/*.h
        .flag("-Wno-int-conversion")
        .flag("-Wno-void-pointer-to-int-cast")
        .flag("-Wno-unused-variable")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-comment");
    if target_os == "emscripten" {{
        build.flag("-fPIC");
    }}

    // ── Version pinning ─────────────────────────────────────────────────
    //
    // With --features pin-version, reads SYNTAQLITE_SQLITE_VERSION env var
    // and passes -DSYNTAQLITE_SQLITE_VERSION=<value> to cc.
    if env::var("CARGO_FEATURE_PIN_VERSION").is_ok() {{
        let ver_str = env::var("SYNTAQLITE_SQLITE_VERSION").unwrap_or_else(|_| {{
            panic!(
                "pin-version feature requires SYNTAQLITE_SQLITE_VERSION env var \
                 (e.g. SYNTAQLITE_SQLITE_VERSION=3035000)"
            )
        }});
        let _: i32 = ver_str.parse().unwrap_or_else(|_| {{
            panic!("SYNTAQLITE_SQLITE_VERSION must be an integer (e.g. 3035000), got: {{ver_str}}")
        }});
        build.define("SYNTAQLITE_SQLITE_VERSION", ver_str.as_str());
    }}

    // ── Cflag pinning ───────────────────────────────────────────────────
    //
    // With --features pin-cflags, scans for SYNTAQLITE_CFLAG_* env vars
    // and passes the same -D flags to cc.
    if env::var("CARGO_FEATURE_PIN_CFLAGS").is_ok() {{
        let all_entries = syntaqlite::dialect::ffi::cflag_table();

        // Pass the master switch.
        build.define("SYNTAQLITE_SQLITE_CFLAGS", None);

        // Scan env vars for SYNTAQLITE_CFLAG_* and pass matching -D flags.
        for entry in all_entries {{
            let env_key = format!("SYNTAQLITE_CFLAG_{{}}", entry.suffix);
            if env::var(&env_key).is_ok() {{
                build.define(&env_key, None);
                println!("cargo:rerun-if-env-changed={{env_key}}");
            }}
        }}
    }}

    build.compile("syntaqlite_dialect");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=csrc");
    println!("cargo:rerun-if-changed=include");
    // Dialect C files #include syntaqlite headers.
    println!("cargo:rerun-if-changed=../syntaqlite-parser-sys/include");
    // Re-run when pinning env vars change.
    println!("cargo:rerun-if-env-changed=SYNTAQLITE_SQLITE_VERSION");
}}
"#
    ));
    w.finish()
}

/// Generate `Cargo.toml` for a dialect crate.
///
/// `crate_name` is the published crate name (e.g. `"syntaqlite"` for the
/// base SQLite dialect, `"syntaqlite-libsql"` for an extension dialect).
pub fn generate_cargo_toml(crate_name: &str) -> String {
    format!(
        r#"# @generated by syntaqlite-buildtools — DO NOT EDIT

[package]
name = "{crate_name}"
version = "0.1.0"
edition = "2024"

[features]
default = ["fmt"]
fmt = ["syntaqlite/fmt"]

# Pin version/cflags at compile time for dead-code elimination.
# Values come from env vars, using the same names as the C defines:
#
#   SYNTAQLITE_SQLITE_VERSION=3035000 cargo build --features pin-version
#
#   SYNTAQLITE_CFLAG_SQLITE_OMIT_WINDOWFUNC=1 \
#   SYNTAQLITE_CFLAG_SQLITE_ENABLE_FTS5=1 \
#   cargo build --features pin-cflags
#
pin-version = []   # Pin SQLite version via SYNTAQLITE_SQLITE_VERSION env var
pin-cflags = []    # Pin compile-time flags via SYNTAQLITE_CFLAG_* env vars

[build-dependencies]
cc = "1.0"
syntaqlite = {{ path = "../syntaqlite", default-features = false }}

[dependencies]
syntaqlite = {{ path = "../syntaqlite", default-features = false }}
"#
    )
}

/// Generate `wrappers.rs` for a dialect crate.
pub fn generate_rust_wrappers() -> String {
    let mut w = RustWriter::new();
    w.file_header();
    emit_section(&mut w, WRAPPERS_PRELUDE);
    emit_section(&mut w, WRAPPER_PARSER);
    emit_section(&mut w, WRAPPER_STATEMENT_CURSOR);
    emit_section(&mut w, WRAPPER_LOW_LEVEL_PARSER);
    emit_section(&mut w, WRAPPER_LOW_LEVEL_CURSOR);
    emit_section(&mut w, WRAPPER_FORMATTER);
    emit_section(&mut w, WRAPPER_TOKENIZER);
    w.lines(WRAPPER_TOKEN_CURSOR);
    w.finish()
}

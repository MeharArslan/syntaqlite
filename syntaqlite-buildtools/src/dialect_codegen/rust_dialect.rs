// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use crate::util::rust_writer::RustWriter;

// ── WrapperContext ────────────────────────────────────────────────────────

/// Controls import paths for the unified wrappers.rs generator.
pub struct WrapperContext<'a> {
    /// Path to the `parser::typed` module, e.g. `"crate::parser::typed"`
    /// (internal) or `"syntaqlite::parser::typed"` (external).
    pub typed_mod: &'a str,
    /// Path to the ast module, e.g. `"syntaqlite_parser_sqlite::ast"`
    /// (internal) or `"crate::ast"` (external).
    pub ast_mod: &'a str,
    /// Path to the tokens module, e.g. `"syntaqlite_parser_sqlite::tokens"`
    /// (internal) or `"crate::tokens"` (external).
    pub tokens_mod: &'a str,
    /// Dialect accessor expression, e.g. `"crate::sqlite::dialect()"`
    /// (internal) or `"crate::dialect()"` (external).
    pub dialect_fn: &'a str,
    /// When `true`, include a `Formatter` wrapper struct that delegates to
    /// `syntaqlite::Formatter`. The internal SQLite crate exports Formatter
    /// directly from `syntaqlite::fmt`, so it doesn't need the wrapper.
    pub include_formatter: bool,
}

impl WrapperContext<'_> {
    pub fn internal_sqlite() -> Self {
        WrapperContext {
            typed_mod: "crate::parser::typed",
            ast_mod: "syntaqlite_parser_sqlite::ast",
            tokens_mod: "syntaqlite_parser_sqlite::tokens",
            dialect_fn: "crate::sqlite::dialect()",
            include_formatter: false,
        }
    }

    pub fn external_dialect() -> Self {
        WrapperContext {
            typed_mod: "syntaqlite::parser::typed",
            ast_mod: "crate::ast",
            tokens_mod: "crate::tokens",
            dialect_fn: "crate::dialect()",
            include_formatter: true,
        }
    }
}

// ── External dialect lib.rs ───────────────────────────────────────────────

const LIB_MODULE_DECLS: &str = r#"
mod ffi;
/// Typed AST nodes for this dialect.
///
/// Each SQL statement type (e.g. `SELECT`, `INSERT`) has a corresponding struct
/// with typed accessors for its fields. The top-level enum is [`ast::Stmt`],
/// returned by [`StatementCursor::next_statement`] and
/// [`RawIncrementalCursor::finish`](low_level::RawIncrementalCursor::finish).
pub mod ast;
mod wrappers;
mod tokens;
"#;

const LIB_LOW_LEVEL_MOD: &str = r#"
/// Low-level APIs for advanced use cases (e.g. custom token feeding/tokenizing).
pub mod low_level {
    pub use crate::wrappers::{IncrementalCursor, IncrementalParser, Token, TokenCursor, Tokenizer};
    pub use crate::tokens::TokenType;
}
"#;

const LIB_EXPORTS: &str = r#"
pub use wrappers::{
    Formatter, IncrementalCursor, IncrementalParser, IncrementalParserBuilder, Parser,
    ParserBuilder, StatementCursor, Token, TokenCursor, Tokenizer, TokenizerBuilder,
};
pub use syntaqlite::ParseError;
"#;

const LIB_CONFIG_MOD: &str = r#"
/// Configuration types for parsers and formatters.
pub mod config {
    pub use syntaqlite::fmt::FormatConfig;
    pub use syntaqlite_parser::dialect::ffi::DialectConfig;
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

use syntaqlite::ext::FfiDialect;
unsafe extern "C" {{
    fn {dialect_fn}() -> *const FfiDialect;
}}

static DIALECT: LazyLock<syntaqlite::Dialect<'static>> =
    LazyLock::new(|| unsafe {{ syntaqlite::Dialect::from_raw({dialect_fn}()) }});

/// Returns the dialect handle.
pub fn dialect() -> syntaqlite::Dialect<'static> {{
    *DIALECT
}}
"#
    ));
    w.newline();
    emit_section(&mut w, LIB_LOW_LEVEL_MOD);
    emit_section(&mut w, LIB_EXPORTS);
    emit_section(&mut w, LIB_CONFIG_MOD);
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
    let runtime_include = manifest_dir.join("../syntaqlite-sys/include");
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
        .include(&runtime_include) // for shared syntaqlite/*.h and syntaqlite_dialect/*.h
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
        let all_entries = syntaqlite::dialect::cflag_table();

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
    println!("cargo:rerun-if-changed=../syntaqlite-sys/include");
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
syntaqlite-parser = {{ path = "../syntaqlite-parser" }}
"#
    )
}

// ── Unified wrappers.rs generator ────────────────────────────────────────

/// Generate `wrappers.rs` for a dialect crate.
///
/// The same template serves both the internal SQLite crate and external dialect
/// crates — only the import paths and dialect accessor differ, as controlled by
/// [`WrapperContext`].
pub fn generate_rust_wrappers(ctx: &WrapperContext<'_>) -> String {
    let typed_mod = ctx.typed_mod;
    let ast_mod = ctx.ast_mod;
    let tokens_mod = ctx.tokens_mod;
    let dialect_fn = ctx.dialect_fn;

    let mut w = RustWriter::new();
    w.file_header();

    if !ctx.include_formatter {
        w.lines(
            r#"//! Thin wrappers around the generic parser/tokenizer types, pre-bound to the
//! SQLite dialect."#,
        );
        w.newline();
    }

    // ── Imports ──────────────────────────────────────────────────────────

    w.lines(&format!(
        r#"
use {typed_mod}::{{
    TypedIncrementalCursor, TypedIncrementalParser, TypedIncrementalParserBuilder, TypedParser,
    TypedParserBuilder, TypedStatementCursor, TypedToken, TypedTokenCursor, TypedTokenizer,
    TypedTokenizerBuilder,
}};
// The dialect is a `'static` singleton, so all dialect-parameterized
// types are concretized to `'static` in this module.
use {ast_mod}::Stmt;
use {tokens_mod}::TokenType;
"#
    ));

    // ── Type aliases ─────────────────────────────────────────────────────

    w.lines(
        r#"
// ── Type aliases ─────────────────────────────────────────────────────────

/// A cursor over parsed SQL statements, yielding typed [`Stmt`] nodes.
pub type StatementCursor<'a> = TypedStatementCursor<'a, Stmt<'a>>;

/// A typed SQL token with kind and source text.
pub type Token<'a> = TypedToken<'a, TokenType>;

/// A cursor yielding typed [`Token`]s.
pub type TokenCursor<'a> = TypedTokenCursor<'a, TokenType>;

/// A tokenizer for SQL.
pub type Tokenizer = TypedTokenizer<'static, TokenType>;

/// Builder for [`Tokenizer`].
pub type TokenizerBuilder = TypedTokenizerBuilder<'static, TokenType>;

/// A cursor for token-by-token incremental parsing.
///
/// Obtained from [`IncrementalParser::feed`] or [`IncrementalParser::feed_cstr`].
/// Feed tokens via [`feed_token`](IncrementalCursor::feed_token) and signal
/// end-of-input via [`finish`](IncrementalCursor::finish).
pub type IncrementalCursor<'a> = TypedIncrementalCursor<'a, Stmt<'a>, TokenType>;
"#,
    );

    // ── Formatter (external only) ─────────────────────────────────────────

    if ctx.include_formatter {
        w.lines(&format!(
            r#"
// ── Formatter ────────────────────────────────────────────────────────────

/// SQL formatter pre-configured for this dialect.
pub struct Formatter {{
    inner: syntaqlite::Formatter<'static>,
}}

impl Formatter {{
    /// Create a formatter with default configuration.
    pub fn new() -> Self {{
        Formatter {{ inner: syntaqlite::Formatter::builder({dialect_fn}).build() }}
    }}

    /// Create a formatter with the given configuration.
    pub fn with_config(config: syntaqlite::fmt::FormatConfig) -> Self {{
        Formatter {{ inner: syntaqlite::Formatter::builder({dialect_fn}).format_config(config).build() }}
    }}

    /// Access the current configuration.
    pub fn config(&self) -> &syntaqlite::fmt::FormatConfig {{
        self.inner.config()
    }}

    /// Format SQL source text.
    pub fn format(&mut self, source: &str) -> Result<String, syntaqlite::ParseError> {{
        self.inner.format(source)
    }}
}}

impl Default for Formatter {{
    fn default() -> Self {{
        Self::new()
    }}
}}
"#
        ));
    }

    // ── Parser ────────────────────────────────────────────────────────────

    w.lines(&format!(
        r#"
// ── Parser ───────────────────────────────────────────────────────────────

/// A SQL parser pre-configured for this dialect.
///
/// Wraps [`TypedParser`] and yields typed [`Stmt`] nodes.
pub struct Parser {{
    inner: TypedParser<'static>,
}}

// SAFETY: TypedParser is Send.
unsafe impl Send for Parser {{}}

impl Parser {{
    /// Create a parser with default configuration.
    pub fn new() -> Self {{
        Parser {{ inner: TypedParser::new({dialect_fn}) }}
    }}

    /// Create a builder for configuring the parser before construction.
    pub fn builder() -> ParserBuilder {{
        ParserBuilder {{ inner: TypedParser::builder({dialect_fn}) }}
    }}

    /// Bind source text and return a [`StatementCursor`] for iterating typed statements.
    pub fn parse<'a>(&'a mut self, source: &'a str) -> StatementCursor<'a> {{
        self.inner.parse(source)
    }}

    /// Zero-copy variant: bind a null-terminated source.
    pub fn parse_cstr<'a>(&'a mut self, source: &'a std::ffi::CStr) -> StatementCursor<'a> {{
        self.inner.parse_cstr(source)
    }}
}}

impl Default for Parser {{
    fn default() -> Self {{
        Self::new()
    }}
}}

// ── ParserBuilder ────────────────────────────────────────────────────────

/// Builder for [`Parser`].
pub struct ParserBuilder {{
    inner: TypedParserBuilder<'static>,
}}

impl ParserBuilder {{
    /// Enable parser trace output.
    pub fn trace(mut self, enable: bool) -> Self {{
        self.inner = self.inner.trace(enable);
        self
    }}

    /// Collect token positions during parsing.
    pub fn collect_tokens(mut self, enable: bool) -> Self {{
        self.inner = self.inner.collect_tokens(enable);
        self
    }}

    /// Set dialect config for version/cflag-gated parsing.
    pub fn dialect_config(mut self, config: syntaqlite_parser::dialect::ffi::DialectConfig) -> Self {{
        self.inner = self.inner.dialect_config(config);
        self
    }}

    /// Build the parser.
    pub fn build(self) -> Parser {{
        Parser {{ inner: self.inner.build() }}
    }}
}}
"#
    ));

    // ── IncrementalParser ─────────────────────────────────────────────────

    w.lines(&format!(
        r#"
// ── IncrementalParser ────────────────────────────────────────────────────

/// An incremental SQL parser pre-configured for this dialect.
///
/// Wraps [`TypedIncrementalParser`] and feeds tokens one at a time via
/// [`IncrementalCursor`], yielding typed [`Stmt`] nodes.
pub struct IncrementalParser {{
    inner: TypedIncrementalParser<'static>,
}}

// SAFETY: TypedIncrementalParser is Send.
unsafe impl Send for IncrementalParser {{}}

impl IncrementalParser {{
    /// Create an incremental parser with default configuration.
    pub fn new() -> Self {{
        IncrementalParser {{ inner: TypedIncrementalParser::new({dialect_fn}) }}
    }}

    /// Create a builder for configuring the parser before construction.
    pub fn builder() -> IncrementalParserBuilder {{
        IncrementalParserBuilder {{ inner: TypedIncrementalParser::builder({dialect_fn}) }}
    }}

    /// Bind source text and return an [`IncrementalCursor`] for token feeding.
    pub fn feed<'a>(&'a mut self, source: &'a str) -> IncrementalCursor<'a> {{
        self.inner.feed(source)
    }}

    /// Zero-copy variant: bind a null-terminated source.
    pub fn feed_cstr<'a>(&'a mut self, source: &'a std::ffi::CStr) -> IncrementalCursor<'a> {{
        self.inner.feed_cstr(source)
    }}
}}

impl Default for IncrementalParser {{
    fn default() -> Self {{
        Self::new()
    }}
}}

// ── IncrementalParserBuilder ─────────────────────────────────────────────

/// Builder for [`IncrementalParser`].
pub struct IncrementalParserBuilder {{
    inner: TypedIncrementalParserBuilder<'static>,
}}

impl IncrementalParserBuilder {{
    /// Enable parser trace output.
    pub fn trace(mut self, enable: bool) -> Self {{
        self.inner = self.inner.trace(enable);
        self
    }}

    /// Collect non-whitespace token positions during parsing.
    pub fn collect_tokens(mut self, enable: bool) -> Self {{
        self.inner = self.inner.collect_tokens(enable);
        self
    }}

    /// Set dialect config for version/cflag-gated parsing.
    pub fn dialect_config(
        mut self,
        config: syntaqlite_parser::dialect::ffi::DialectConfig,
    ) -> Self {{
        self.inner = self.inner.dialect_config(config);
        self
    }}

    /// Build the parser.
    pub fn build(self) -> IncrementalParser {{
        IncrementalParser {{ inner: self.inner.build() }}
    }}
}}
"#
    ));

    w.finish()
}

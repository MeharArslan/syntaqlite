// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![expect(
    missing_docs,
    reason = "bin crate; internal lib shim exists only to support integration tests"
)]

use clap::{Parser, Subcommand, ValueEnum};

#[cfg(feature = "builtin-sqlite")]
mod runtime;

#[cfg(feature = "builtin-sqlite")]
mod codegen;

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum ParseOutput {
    /// Print statement/error counts (compact, for benchmarks)
    Summary,
    /// Print the full AST
    Ast,
}

#[derive(Parser)]
#[command(
    name = "syntaqlite",
    about = "SQL formatting and analysis tools",
    version
)]
pub(crate) struct Cli {
    /// Path to a shared library (.so/.dylib/.dll) providing a dialect.
    #[cfg(feature = "builtin-sqlite")]
    #[arg(long = "dialect")]
    pub(crate) dialect_path: Option<String>,

    /// Dialect name for symbol lookup.
    /// When omitted, the loader resolves `syntaqlite_grammar`.
    /// With a name, it resolves `syntaqlite_<name>_grammar`.
    #[cfg(feature = "builtin-sqlite")]
    #[arg(long, requires = "dialect_path")]
    pub(crate) dialect_name: Option<String>,

    /// `SQLite` version to emulate (e.g. "3.47.0", "latest").
    #[cfg(feature = "builtin-sqlite")]
    #[arg(long)]
    pub(crate) sqlite_version: Option<String>,

    /// Enable a `SQLite` compile-time flag (e.g. `SQLITE_ENABLE_ORDERED_SET_AGGREGATES`).
    /// Can be specified multiple times.
    #[cfg(feature = "builtin-sqlite")]
    #[arg(long)]
    pub(crate) sqlite_cflag: Vec<String>,

    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Parse SQL and report results
    #[cfg(feature = "builtin-sqlite")]
    Parse {
        /// SQL files or glob patterns (reads stdin if omitted)
        files: Vec<String>,
        /// Output format
        #[arg(short, long, value_enum, default_value_t = ParseOutput::Summary)]
        output: ParseOutput,
    },
    /// Format SQL
    #[cfg(feature = "builtin-sqlite")]
    Fmt {
        /// SQL files or glob patterns (reads stdin if omitted)
        files: Vec<String>,
        /// Maximum line width
        #[arg(short = 'w', long, default_value_t = 80)]
        line_width: usize,
        /// Spaces per indentation level
        #[arg(short = 't', long, default_value_t = 2)]
        indent_width: usize,
        /// Keyword casing
        #[arg(short = 'k', long, value_enum, default_value_t = runtime::KeywordCasing::Upper)]
        keyword_case: runtime::KeywordCasing,
        /// Write formatted output back to file(s) in place
        #[arg(short = 'i', long)]
        in_place: bool,
        /// Check if files are formatted (exit 1 if not)
        #[arg(long, conflicts_with = "in_place")]
        check: bool,
        /// Append semicolons after each statement
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        semicolons: bool,
    },
    /// Validate SQL and report diagnostics
    #[cfg(feature = "builtin-sqlite")]
    Validate {
        /// SQL files or glob patterns (reads stdin if omitted)
        files: Vec<String>,
        /// Schema DDL file(s) to load before validation (repeatable, supports globs)
        #[arg(long)]
        schema: Vec<String>,
        /// [experimental] Host language for embedded SQL extraction (python, typescript)
        #[arg(long = "experimental-lang")]
        lang: Option<runtime::HostLanguage>,
    },
    /// Start the language server (stdio)
    #[cfg(feature = "builtin-sqlite")]
    Lsp,
    /// Generate dialect C sources and Rust bindings for external dialects.
    #[cfg(feature = "builtin-sqlite")]
    Dialect(codegen::DialectArgs),
    /// Print version information
    Version,
    #[cfg(feature = "builtin-sqlite")]
    #[command(flatten)]
    DialectTool(codegen::ToolCommand),
}

/// Run the CLI.
#[cfg(feature = "builtin-sqlite")]
pub fn run(name: &str, dialect: Option<syntaqlite::any::AnyDialect>) {
    let cli =
        Cli::try_parse_from(std::iter::once(name.to_string()).chain(std::env::args().skip(1)))
            .unwrap_or_else(|e| e.exit());

    let result = runtime::dispatch(cli, dialect);

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

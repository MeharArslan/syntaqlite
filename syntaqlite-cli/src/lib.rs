// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![expect(
    missing_docs,
    reason = "bin crate; internal lib shim exists only to support integration tests"
)]
#![cfg_attr(test, expect(clippy::unwrap_used, clippy::similar_names))]

use clap::{Parser, Subcommand};

#[cfg(feature = "builtin-sqlite")]
mod runtime;

#[cfg(feature = "builtin-sqlite")]
mod codegen;

#[derive(Parser)]
#[command(about = "SQL formatting and analysis tools")]
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

    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Parse SQL and print the AST
    #[cfg(feature = "builtin-sqlite")]
    Ast {
        /// SQL files or glob patterns (reads stdin if omitted)
        files: Vec<String>,
    },
    /// Format SQL
    #[cfg(feature = "builtin-sqlite")]
    Fmt {
        /// SQL files or glob patterns (reads stdin if omitted)
        files: Vec<String>,
        /// Maximum line width
        #[arg(short = 'w', long, default_value_t = 80)]
        line_width: usize,
        /// Keyword casing
        #[arg(short = 'k', long, value_enum, default_value_t = runtime::KeywordCasing::Upper)]
        keyword_case: runtime::KeywordCasing,
        /// Write formatted output back to file(s) in place
        #[arg(short = 'i', long)]
        in_place: bool,
        /// Append semicolons after each statement
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        semicolons: bool,
    },
    /// Validate SQL and report diagnostics
    #[cfg(feature = "builtin-sqlite")]
    Validate {
        /// SQL files or glob patterns (reads stdin if omitted)
        files: Vec<String>,
        /// Host language for embedded SQL extraction (python, typescript)
        #[arg(long)]
        lang: Option<runtime::HostLanguage>,
    },
    /// Start the language server (stdio)
    #[cfg(feature = "builtin-sqlite")]
    Lsp,
    /// Generate dialect C sources and Rust bindings for external dialects.
    #[cfg(feature = "builtin-sqlite")]
    Dialect(codegen::DialectArgs),
    /// Hidden lemon/mkkeyword subcommands for codegen subprocess support.
    #[cfg(feature = "builtin-sqlite")]
    #[command(flatten)]
    DialectTool(codegen::ToolCommand),
}

/// Run the CLI.
#[cfg(feature = "builtin-sqlite")]
pub fn run(name: &str, dialect: Option<syntaqlite::Dialect>) {
    let cli =
        Cli::try_parse_from(std::iter::once(name.to_string()).chain(std::env::args().skip(1)))
            .unwrap_or_else(|e| e.exit());

    let result = runtime::dispatch(cli, dialect);

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

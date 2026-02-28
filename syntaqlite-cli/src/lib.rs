// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use clap::{Parser, Subcommand};

#[cfg(feature = "runtime")]
mod runtime;

#[cfg(any(feature = "codegen-dialect", feature = "codegen-sqlite"))]
mod fs_util;

#[cfg(feature = "codegen-dialect")]
mod codegen_dialect;

#[cfg(feature = "codegen-sqlite")]
mod codegen_sqlite;

#[cfg(feature = "runtime")]
mod lsp;

#[cfg(feature = "sqlite-extract")]
mod sqlite_extract;

#[cfg(feature = "version-analysis")]
mod version_analysis;

#[derive(Parser)]
#[command(about = "SQL formatting and analysis tools")]
struct Cli {
    /// Path to a shared library (.so/.dylib/.dll) providing a dialect.
    #[cfg(feature = "runtime")]
    #[arg(long = "dialect")]
    dialect_path: Option<String>,

    /// Dialect name for symbol lookup.
    /// When omitted, the loader resolves `syntaqlite_dialect`.
    /// With a name, it resolves `syntaqlite_<name>_dialect`.
    #[cfg(feature = "runtime")]
    #[arg(long, requires = "dialect_path")]
    dialect_name: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse SQL and print the AST
    #[cfg(feature = "runtime")]
    Ast {
        /// SQL files or glob patterns (reads stdin if omitted)
        files: Vec<String>,
    },
    /// Format SQL
    #[cfg(feature = "runtime")]
    Fmt {
        /// SQL files or glob patterns (reads stdin if omitted)
        files: Vec<String>,
        /// Maximum line width
        #[arg(short = 'w', long, default_value_t = 80)]
        line_width: usize,
        /// Keyword casing
        #[arg(short = 'k', long, value_enum, default_value_t = runtime::CasingArg::Upper)]
        keyword_case: runtime::CasingArg,
        /// Write formatted output back to file(s) in place
        #[arg(short = 'i', long)]
        in_place: bool,
        /// Append semicolons after each statement
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        semicolons: bool,
    },
    /// Start the language server (stdio)
    #[cfg(feature = "runtime")]
    Lsp,
    #[cfg(feature = "codegen-dialect")]
    #[command(flatten)]
    Dialect(codegen_dialect::CodegenCommand),
    #[cfg(feature = "codegen-sqlite")]
    #[command(flatten)]
    Sqlite(codegen_sqlite::CodegenCommand),
    #[cfg(feature = "sqlite-extract")]
    #[command(flatten)]
    Extract(sqlite_extract::ExtractCommand),
    #[cfg(feature = "version-analysis")]
    #[command(flatten)]
    VersionAnalysis(version_analysis::VersionAnalysisCommand),
}

/// Run the CLI with the given dialect configuration.
///
/// `dialect` is `None` when built without `builtin-sqlite` — runtime commands
/// (ast, fmt, lsp) will error, but codegen commands work fine.
#[cfg(feature = "runtime")]
pub fn run(name: &str, dialect: Option<&syntaqlite::Dialect>) {
    let cli =
        Cli::try_parse_from(std::iter::once(name.to_string()).chain(std::env::args().skip(1)))
            .unwrap_or_else(|e| e.exit());

    let result = runtime::dispatch(cli, dialect);

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

/// Run the CLI without runtime dialect support (extract/codegen only).
#[cfg(not(feature = "runtime"))]
pub fn run(name: &str, _dialect: Option<()>) {
    let cli =
        Cli::try_parse_from(std::iter::once(name.to_string()).chain(std::env::args().skip(1)))
            .unwrap_or_else(|e| e.exit());

    let result = match cli.command {
        #[cfg(feature = "codegen-dialect")]
        Command::Dialect(cmd) => codegen_dialect::dispatch(cmd),
        #[cfg(feature = "codegen-sqlite")]
        Command::Sqlite(cmd) => codegen_sqlite::dispatch(cmd),
        #[cfg(feature = "sqlite-extract")]
        Command::Extract(cmd) => sqlite_extract::dispatch(cmd),
        #[cfg(feature = "version-analysis")]
        Command::VersionAnalysis(cmd) => version_analysis::dispatch(cmd),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "runtime")]
    use super::runtime::dialect_symbol_name;

    #[test]
    #[cfg(feature = "runtime")]
    fn picks_default_symbol_when_name_missing() {
        assert_eq!(dialect_symbol_name(None), "syntaqlite_dialect");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn uses_named_symbol_when_name_given() {
        assert_eq!(
            dialect_symbol_name(Some("sqlite")),
            "syntaqlite_sqlite_dialect"
        );
    }
}

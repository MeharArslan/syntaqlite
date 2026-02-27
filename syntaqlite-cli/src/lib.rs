// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use syntaqlite_runtime::dialect::ffi as dialect_ffi;
use syntaqlite_runtime::fmt::{FormatConfig, Formatter, KeywordCase};
use syntaqlite_runtime::{Dialect, ParseError, Parser as RuntimeParser};

#[cfg(feature = "codegen-dialect")]
mod codegen_dialect;

#[cfg(feature = "codegen-sqlite")]
mod codegen_sqlite;

mod lsp;

#[cfg(feature = "sqlite-extract")]
mod sqlite_extract;

#[cfg(feature = "version-analysis")]
mod version_analysis;

#[derive(Parser)]
#[command(about = "SQL formatting and analysis tools")]
struct Cli {
    /// Path to a shared library (.so/.dylib/.dll) providing a dialect.
    #[arg(long = "dialect")]
    dialect_path: Option<String>,

    /// Dialect name for symbol lookup.
    /// When omitted, the loader resolves `syntaqlite_dialect`.
    /// With a name, it resolves `syntaqlite_<name>_dialect`.
    #[arg(long, requires = "dialect_path")]
    dialect_name: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse SQL and print the AST
    Ast {
        /// SQL files or glob patterns (reads stdin if omitted)
        files: Vec<String>,
    },
    /// Format SQL
    Fmt {
        /// SQL files or glob patterns (reads stdin if omitted)
        files: Vec<String>,
        /// Maximum line width
        #[arg(short = 'w', long, default_value_t = 80)]
        line_width: usize,
        /// Keyword casing
        #[arg(short = 'k', long, value_enum, default_value_t = CasingArg::Upper)]
        keyword_case: CasingArg,
        /// Write formatted output back to file(s) in place
        #[arg(short = 'i', long)]
        in_place: bool,
        /// Append semicolons after each statement
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        semicolons: bool,
    },
    /// Start the language server (stdio)
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

#[derive(Clone, Copy, ValueEnum)]
enum CasingArg {
    Preserve,
    Upper,
    Lower,
}

/// Expand a list of file paths / glob patterns into concrete paths.
/// Returns an empty vec when the input is empty (meaning: read stdin).
fn expand_paths(patterns: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for pat in patterns {
        let matches: Vec<_> = glob::glob(pat)
            .map_err(|e| format!("bad glob pattern {pat:?}: {e}"))?
            .collect();
        if matches.is_empty() {
            return Err(format!("no files matched: {pat}"));
        }
        for entry in matches {
            let path = entry.map_err(|e| format!("glob error: {e}"))?;
            if path.is_file() {
                out.push(path);
            }
        }
    }
    Ok(out)
}

fn require_dialect<'a>(dialect: Option<&'a Dialect<'a>>) -> Result<&'a Dialect<'a>, String> {
    dialect.ok_or_else(|| {
        "this command requires a dialect; build with --features=builtin-sqlite or use --dialect"
            .to_string()
    })
}

fn cmd_ast(dialect: &Dialect, files: Vec<String>) -> Result<(), String> {
    let paths = expand_paths(&files)?;

    if paths.is_empty() {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("reading stdin: {e}"))?;
        return cmd_ast_source(dialect, &buf);
    }

    for path in &paths {
        let source = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if paths.len() > 1 {
            println!("==> {} <==", path.display());
        }
        cmd_ast_source(dialect, &source)?;
    }
    Ok(())
}

fn cmd_ast_source(dialect: &Dialect, source: &str) -> Result<(), String> {
    let buf = dump_ast_source(dialect, source).map_err(|e| format!("parse error: {e}"))?;
    print!("{buf}");
    Ok(())
}

fn cmd_fmt(
    dialect: &Dialect,
    files: Vec<String>,
    config: FormatConfig,
    in_place: bool,
    semicolons: bool,
) -> Result<(), String> {
    let mut config = config;
    config.semicolons = semicolons;

    let paths = expand_paths(&files)?;

    if paths.is_empty() {
        if in_place {
            return Err("--in-place requires file arguments".to_string());
        }
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .map_err(|e| format!("reading stdin: {e}"))?;
        let out = format_source(dialect, &source, config.clone()).map_err(|e| format!("{e}"))?;
        print!("{out}");
        return Ok(());
    }

    let mut errors = Vec::new();
    for path in &paths {
        let source = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        match format_source(dialect, &source, config.clone()) {
            Ok(out) => {
                if in_place {
                    if out != source {
                        fs::write(path, &out).map_err(|e| format!("{}: {e}", path.display()))?;
                        eprintln!("formatted {}", path.display());
                    }
                } else {
                    if paths.len() > 1 {
                        println!("==> {} <==", path.display());
                    }
                    print!("{out}");
                }
            }
            Err(e) => {
                errors.push(format!("{}: {e}", path.display()));
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    Ok(())
}

fn dump_ast_source(dialect: &Dialect, source: &str) -> Result<String, ParseError> {
    let mut parser = RuntimeParser::new(dialect);
    let mut cursor = parser.parse(source);
    let mut out = String::new();
    let mut count = 0;

    while let Some(result) = cursor.next_statement() {
        let root_id = result?;
        if count > 0 {
            out.push_str("----\n");
        }
        cursor.dump_node(root_id, &mut out, 0);
        count += 1;
    }

    Ok(out)
}

fn format_source(
    dialect: &Dialect,
    source: &str,
    config: FormatConfig,
) -> Result<String, ParseError> {
    let mut formatter = Formatter::with_config(dialect, config).map_err(|e| ParseError {
        message: e.to_string(),
        offset: None,
        length: None,
    })?;
    formatter.format(source)
}

const DEFAULT_DIALECT_SYMBOL: &str = "syntaqlite_dialect";

fn dialect_symbol_name(name: Option<&str>) -> String {
    match name {
        Some(name) => format!("syntaqlite_{name}_dialect"),
        None => DEFAULT_DIALECT_SYMBOL.to_string(),
    }
}

/// Load an extension dialect from a shared library.
///
/// The library must export `syntaqlite_dialect` by default, or
/// `syntaqlite_<name>_dialect` when `name` is provided. The returned
/// `Library` must be kept alive for the `Dialect` to remain valid.
unsafe fn load_dynamic_dialect(
    path: &str,
    name: Option<&str>,
) -> Result<(libloading::Library, Dialect<'static>), String> {
    let lib = unsafe {
        libloading::Library::new(path).map_err(|e| format!("failed to load {path}: {e}"))?
    };

    let symbol_name = dialect_symbol_name(name);
    let func: libloading::Symbol<unsafe extern "C" fn() -> *const dialect_ffi::Dialect> = unsafe {
        lib.get(symbol_name.as_bytes())
            .map_err(|e| format!("symbol {symbol_name} not found in {path}: {e}"))?
    };

    let raw = unsafe { func() };
    if raw.is_null() {
        return Err(format!("{symbol_name} returned null"));
    }
    let dialect = unsafe { Dialect::from_raw(raw) };

    Ok((lib, dialect))
}

/// Run the CLI with the given dialect configuration.
///
/// `dialect` is `None` when built without `builtin-sqlite` — runtime commands
/// (ast, fmt, lsp) will error, but codegen commands work fine.
pub fn run(name: &str, dialect: Option<&Dialect>) {
    let cli =
        Cli::try_parse_from(std::iter::once(name.to_string()).chain(std::env::args().skip(1)))
            .unwrap_or_else(|e| e.exit());

    // Load a dynamic dialect if requested. The library handle must stay alive
    // until after the command finishes.
    let _dialect_lib;
    let dyn_dialect;
    let active_dialect: Option<&Dialect>;

    if let Some(path) = &cli.dialect_path {
        let (lib, d) = unsafe { load_dynamic_dialect(path, cli.dialect_name.as_deref()) }
            .unwrap_or_else(|e| {
                eprintln!("error: {e}");
                std::process::exit(1);
            });
        _dialect_lib = Some(lib);
        dyn_dialect = d;
        active_dialect = Some(&dyn_dialect);
    } else {
        _dialect_lib = None;
        active_dialect = dialect;
    }

    let result = match cli.command {
        Command::Ast { files } => require_dialect(active_dialect).and_then(|d| cmd_ast(d, files)),
        Command::Lsp => require_dialect(active_dialect).and_then(|d| lsp::cmd_lsp(d)),
        Command::Fmt {
            files,
            line_width,
            keyword_case,
            in_place,
            semicolons,
        } => {
            let config = FormatConfig {
                line_width,
                keyword_case: match keyword_case {
                    CasingArg::Preserve => KeywordCase::Preserve,
                    CasingArg::Upper => KeywordCase::Upper,
                    CasingArg::Lower => KeywordCase::Lower,
                },
                ..Default::default()
            };
            require_dialect(active_dialect)
                .and_then(|d| cmd_fmt(d, files, config, in_place, semicolons))
        }
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
    use super::dialect_symbol_name;

    #[test]
    fn picks_default_symbol_when_name_missing() {
        assert_eq!(dialect_symbol_name(None), "syntaqlite_dialect");
    }

    #[test]
    fn uses_named_symbol_when_name_given() {
        assert_eq!(
            dialect_symbol_name(Some("sqlite")),
            "syntaqlite_sqlite_dialect"
        );
    }
}

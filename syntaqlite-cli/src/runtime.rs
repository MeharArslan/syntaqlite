// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Runtime SQL commands (ast, fmt, lsp) that require the `syntaqlite` crate.

use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use clap::ValueEnum;
use syntaqlite::dialect::ffi as dialect_ffi;
use syntaqlite::fmt::{FormatConfig, Formatter, KeywordCase};
use syntaqlite::validation::{Severity, ValidationConfig};
use syntaqlite::{Dialect, ParseError, Parser as RuntimeParser};

use super::{Cli, Command};

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum CasingArg {
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

const DEFAULT_DIALECT_SYMBOL: &str = "syntaqlite_dialect";

pub(crate) fn dialect_symbol_name(name: Option<&str>) -> String {
    match name {
        Some(name) => format!("syntaqlite_{name}_dialect"),
        None => DEFAULT_DIALECT_SYMBOL.to_string(),
    }
}

/// Load an extension dialect from a shared library.
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

pub(crate) fn dispatch(cli: Cli, dialect: Option<&Dialect>) -> Result<(), String> {
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

    match cli.command {
        Command::Ast { files } => require_dialect(active_dialect).and_then(|d| cmd_ast(d, files)),
        Command::Validate { files } => {
            require_dialect(active_dialect).and_then(|d| cmd_validate(d, files))
        }
        Command::Lsp => require_dialect(active_dialect).and_then(|d| crate::lsp::cmd_lsp(d)),
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
        Command::Dialect(cmd) => crate::codegen_dialect::dispatch(cmd),
        #[cfg(feature = "codegen-sqlite")]
        Command::Sqlite(cmd) => crate::codegen_sqlite::dispatch(cmd),
        #[cfg(feature = "sqlite-extract")]
        Command::Extract(cmd) => crate::sqlite_extract::dispatch(cmd),
        #[cfg(feature = "version-analysis")]
        Command::VersionAnalysis(cmd) => crate::version_analysis::dispatch(cmd),
    }
}

fn read_stdin() -> Result<String, String> {
    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| format!("reading stdin: {e}"))?;
    Ok(buf)
}

fn cmd_ast(dialect: &Dialect, files: Vec<String>) -> Result<(), String> {
    let paths = expand_paths(&files)?;

    if paths.is_empty() {
        return cmd_ast_source(dialect, &read_stdin()?);
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
        let source = read_stdin()?;
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
    let mut parser = RuntimeParser::with_dialect(dialect);
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
    let mut formatter =
        Formatter::with_dialect_config(dialect, config).map_err(|e| ParseError {
            message: e.to_string(),
            offset: None,
            length: None,
        })?;
    formatter.format(source)
}

fn cmd_validate(dialect: &Dialect, files: Vec<String>) -> Result<(), String> {
    let paths = expand_paths(&files)?;
    let config = ValidationConfig::default();

    if paths.is_empty() {
        let source = read_stdin()?;
        let has_errors = validate_source(dialect, &source, &config);
        if has_errors {
            std::process::exit(1);
        }
        return Ok(());
    }

    let mut any_errors = false;
    for path in &paths {
        let source = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if paths.len() > 1 {
            println!("==> {} <==", path.display());
        }
        if validate_source(dialect, &source, &config) {
            any_errors = true;
        }
    }
    if any_errors {
        std::process::exit(1);
    }
    Ok(())
}

/// Validate a source string and print diagnostics. Returns `true` if any errors were found.
fn validate_source(dialect: &Dialect, source: &str, config: &ValidationConfig) -> bool {
    let mut parser = RuntimeParser::with_dialect(dialect);
    let mut cursor = parser.parse(source);

    let stmt_ids: Vec<_> = (&mut cursor).map_while(|r| r.ok()).collect();
    let diags =
        syntaqlite::validation::validate_document(cursor.reader(), &stmt_ids, dialect, None, &[], config);

    let mut has_errors = false;
    for d in &diags {
        let severity = match d.severity {
            Severity::Error => {
                has_errors = true;
                "error"
            }
            Severity::Warning => "warning",
            Severity::Info => "info",
            Severity::Hint => "hint",
        };
        println!("{severity} {}..{}: {}", d.start_offset, d.end_offset, d.message);
    }

    has_errors
}

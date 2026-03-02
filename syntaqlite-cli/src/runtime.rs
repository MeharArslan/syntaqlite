// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Runtime SQL commands (ast, fmt, lsp, validate) that require the `syntaqlite` crate.

use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use clap::ValueEnum;
use syntaqlite::Formatter;
use syntaqlite::validation::{SourceContext, ValidationConfig};
use syntaqlite::{FormatConfig, KeywordCase};
use syntaqlite_parser::{FfiDialect, ParseError, RawDialect, RawParser};

use super::{Cli, Command};

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum KeywordCasing {
    Preserve,
    Upper,
    Lower,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum HostLanguage {
    Python,
    Typescript,
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

fn require_dialect(dialect: Option<RawDialect<'_>>) -> Result<RawDialect<'_>, String> {
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

/// Load a dialect symbol from an already-open shared library.
///
/// The returned `RawDialect<'lib>` borrows from `lib` and must not outlive it.
///
/// # Safety
/// `lib` must remain valid for the lifetime `'lib` of the returned dialect.
unsafe fn dialect_from_library<'lib>(
    lib: &'lib libloading::Library,
    name: Option<&str>,
) -> Result<RawDialect<'lib>, String> {
    let symbol_name = dialect_symbol_name(name);
    let func: libloading::Symbol<unsafe extern "C" fn() -> *const FfiDialect> = unsafe {
        lib.get(symbol_name.as_bytes())
            .map_err(|e| format!("symbol {symbol_name} not found in library: {e}"))?
    };
    let raw = unsafe { func() };
    if raw.is_null() {
        return Err(format!("{symbol_name} returned null"));
    }
    Ok(unsafe { RawDialect::from_raw(raw) })
}

pub(crate) fn dispatch(cli: Cli, dialect: Option<RawDialect<'_>>) -> Result<(), String> {
    if let Some(path) = &cli.dialect_path {
        // lib must be declared before dyn_dialect so Rust's reverse drop order
        // ensures dyn_dialect is dropped before lib (which would unload the library).
        let lib = unsafe {
            libloading::Library::new(path).map_err(|e| format!("failed to load {path}: {e}"))
        }
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            std::process::exit(1);
        });
        let dyn_dialect = unsafe { dialect_from_library(&lib, cli.dialect_name.as_deref()) }
            .unwrap_or_else(|e| {
                eprintln!("error: {e}");
                std::process::exit(1);
            });
        dispatch_commands(cli.command, Some(dyn_dialect))
    } else {
        dispatch_commands(cli.command, dialect)
    }
}

fn dispatch_commands(command: Command, dialect: Option<RawDialect<'_>>) -> Result<(), String> {
    match command {
        Command::Ast { files } => require_dialect(dialect).and_then(|d| cmd_ast(d, files)),
        Command::Validate { files, lang } => {
            require_dialect(dialect).and_then(|d| cmd_validate(d, files, lang))
        }
        Command::Lsp => require_dialect(dialect).and_then(|d| {
            syntaqlite::lsp::LspServer::run(d).map_err(|e| format!("LSP error: {e}"))
        }),
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
                    KeywordCasing::Preserve => KeywordCase::Preserve,
                    KeywordCasing::Upper => KeywordCase::Upper,
                    KeywordCasing::Lower => KeywordCase::Lower,
                },
                semicolons,
                ..Default::default()
            };
            require_dialect(dialect).and_then(|d| cmd_fmt(d, files, config, in_place))
        }
        #[cfg(feature = "codegen-dialect")]
        Command::CodegenDialect(args) => crate::codegen::dispatch_dialect(args),
        #[cfg(feature = "codegen-dialect")]
        Command::DialectTool(cmd) => crate::codegen::dispatch_tool(cmd),
        #[cfg(feature = "internal")]
        Command::CodegenSqliteParser(args) => crate::codegen::dispatch_sqlite_parser(args),
        #[cfg(feature = "sqlite-extract")]
        Command::Extract(cmd) => crate::extract::dispatch_extract(cmd),
        #[cfg(feature = "version-analysis")]
        Command::VersionAnalysis(cmd) => crate::extract::dispatch_version_analysis(cmd),
    }
}

fn read_stdin() -> Result<String, String> {
    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| format!("reading stdin: {e}"))?;
    Ok(buf)
}

/// Expand file patterns and dispatch to `on_stdin` (no files) or `on_file` (each file).
///
/// Handles glob expansion and reading each file. The `on_file` closure receives
/// `(source, path, multi)` where `multi` is `true` when processing multiple files.
fn process_files(
    files: Vec<String>,
    on_stdin: impl FnOnce(&str) -> Result<(), String>,
    mut on_file: impl FnMut(&str, &PathBuf, bool) -> Result<(), String>,
) -> Result<(), String> {
    let paths = expand_paths(&files)?;

    if paths.is_empty() {
        return on_stdin(&read_stdin()?);
    }

    let multi = paths.len() > 1;
    for path in &paths {
        let source = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        on_file(&source, path, multi)?;
    }
    Ok(())
}

fn cmd_ast(dialect: RawDialect<'_>, files: Vec<String>) -> Result<(), String> {
    process_files(
        files,
        |source| cmd_ast_source(dialect, source, "<stdin>"),
        |source, path, multi| {
            let file = path.display().to_string();
            if multi {
                println!("==> {file} <==");
            }
            cmd_ast_source(dialect, source, &file)
        },
    )
}

fn cmd_ast_source(dialect: RawDialect<'_>, source: &str, file: &str) -> Result<(), String> {
    let (buf, errors) = dump_ast_source(dialect, source);
    print!("{buf}");
    if errors.is_empty() {
        Ok(())
    } else {
        let ctx = SourceContext::new(source, file);
        for e in &errors {
            let start = e.offset.unwrap_or(0);
            let end = start + e.length.unwrap_or(0);
            ctx.render_diagnostic("error", &e.message, start, end, None);
        }
        Err(format!("{} syntax error(s)", errors.len()))
    }
}

fn cmd_fmt(
    dialect: RawDialect<'_>,
    files: Vec<String>,
    config: FormatConfig,
    in_place: bool,
) -> Result<(), String> {
    let mut errors = Vec::new();
    process_files(
        files,
        |source| {
            if in_place {
                return Err("--in-place requires file arguments".to_string());
            }
            let out = format_source(dialect, source, config.clone()).map_err(|e| format!("{e}"))?;
            print!("{out}");
            Ok(())
        },
        |source, path, multi| {
            match format_source(dialect, source, config.clone()) {
                Ok(out) => {
                    if in_place {
                        if out != source {
                            fs::write(path, &out)
                                .map_err(|e| format!("{}: {e}", path.display()))?;
                            eprintln!("formatted {}", path.display());
                        }
                    } else {
                        if multi {
                            println!("==> {} <==", path.display());
                        }
                        print!("{out}");
                    }
                }
                Err(e) => {
                    errors.push(format!("{}: {e}", path.display()));
                }
            }
            Ok(())
        },
    )?;

    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    Ok(())
}

fn dump_ast_source(dialect: RawDialect<'_>, source: &str) -> (String, Vec<ParseError>) {
    let mut parser = RawParser::new(dialect);
    let mut cursor = parser.parse(source);
    let mut out = String::new();
    let mut errors = Vec::new();
    let mut count = 0;

    while let Some(result) = cursor.next_statement() {
        match result {
            Ok(node) => {
                if count > 0 {
                    out.push_str("----\n");
                }
                node.dump(&mut out, 0);
                count += 1;
            }
            Err(err) => errors.push(err),
        }
    }

    (out, errors)
}

fn format_source(
    dialect: RawDialect<'_>,
    source: &str,
    config: FormatConfig,
) -> Result<String, ParseError> {
    let mut formatter = Formatter::with_config(dialect, &config, None);
    formatter.format(source)
}

fn cmd_validate(
    dialect: RawDialect<'_>,
    files: Vec<String>,
    lang: Option<HostLanguage>,
) -> Result<(), String> {
    let config = ValidationConfig::default();
    let mut any_errors = false;

    let validate = |source: &str, file: &str| -> bool {
        match lang {
            Some(lang) => validate_embedded_source(dialect, source, file, &config, lang),
            None => validate_source(dialect, source, file, &config),
        }
    };

    process_files(
        files,
        |source| {
            if validate(source, "<stdin>") {
                std::process::exit(1);
            }
            Ok(())
        },
        |source, path, multi| {
            let file = path.display().to_string();
            if multi {
                println!("==> {file} <==");
            }
            if validate(source, &file) {
                any_errors = true;
            }
            Ok(())
        },
    )?;

    if any_errors {
        std::process::exit(1);
    }
    Ok(())
}

fn validate_source(
    dialect: RawDialect<'_>,
    source: &str,
    file: &str,
    config: &ValidationConfig,
) -> bool {
    let functions = syntaqlite::embedded::sqlite_function_defs();
    let mut validator = syntaqlite::Validator::with_config(dialect, functions, None);
    let diags = validator.validate(source, None, config);
    SourceContext::new(source, file).render_diagnostics(&diags)
}

fn validate_embedded_source(
    dialect: RawDialect<'_>,
    source: &str,
    file: &str,
    config: &ValidationConfig,
    lang: HostLanguage,
) -> bool {
    let fragments = match lang {
        HostLanguage::Python => syntaqlite::embedded::extract_python(source),
        HostLanguage::Typescript => syntaqlite::embedded::extract_typescript(source),
    };
    if fragments.is_empty() {
        eprintln!("no SQL fragments found in {file}");
        return false;
    }

    let diags = syntaqlite::embedded::EmbeddedAnalyzer::new(dialect)
        .with_functions(syntaqlite::embedded::sqlite_function_defs())
        .with_config(*config)
        .validate(&fragments);

    SourceContext::new(source, file).render_diagnostics(&diags)
}

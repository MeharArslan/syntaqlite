// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Runtime SQL commands (ast, fmt, lsp, validate) that require the `syntaqlite` crate.

use std::fs;
use std::io::{self, Read};
use std::ops::Deref;
use std::path::PathBuf;

use clap::ValueEnum;
use syntaqlite::any::{AnyParser, ParseOutcome};
use syntaqlite::{
    Catalog, Diagnostic, DiagnosticMessage, DiagnosticRenderer, FormatConfig, Formatter,
    KeywordCase, SemanticAnalyzer, Severity, ValidationConfig,
};
use syntaqlite::{Dialect, FormatError};

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

fn require_dialect(dialect: Option<Dialect>) -> Result<Dialect, String> {
    dialect.ok_or_else(|| {
        "this command requires a dialect; build with --features=builtin-sqlite or use --dialect"
            .to_string()
    })
}

pub(crate) fn dispatch(cli: Cli, dialect: Option<Dialect>) -> Result<(), String> {
    if let Some(path) = &cli.dialect_path {
        let dyn_dialect = Dialect::load(path, cli.dialect_name.as_deref())
            .unwrap_or_else(|e| {
                eprintln!("error: {e}");
                std::process::exit(1);
            });
        dispatch_commands(cli.command, Some(dyn_dialect))
    } else {
        dispatch_commands(cli.command, dialect)
    }
}

fn dispatch_commands(command: Command, dialect: Option<Dialect>) -> Result<(), String> {
    match command {
        Command::Ast { files } => require_dialect(dialect).and_then(|d| cmd_ast(d, files)),
        Command::Validate { files, lang } => {
            require_dialect(dialect).and_then(|d| cmd_validate(d, files, lang))
        }
        Command::Lsp => require_dialect(dialect)
            .and_then(|d| syntaqlite::LspServer::run(d).map_err(|e| format!("LSP error: {e}"))),
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
        Command::Dialect(args) => crate::codegen::dispatch_dialect(args),
        Command::DialectTool(cmd) => crate::codegen::dispatch_tool(cmd),
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

fn cmd_ast(dialect: Dialect, files: Vec<String>) -> Result<(), String> {
    process_files(
        files,
        |source| cmd_ast_source(&dialect, source, "<stdin>"),
        |source, path, multi| {
            let file = path.display().to_string();
            if multi {
                println!("==> {file} <==");
            }
            cmd_ast_source(&dialect, source, &file)
        },
    )
}

fn cmd_ast_source(dialect: &Dialect, source: &str, file: &str) -> Result<(), String> {
    let parser = AnyParser::new(*dialect.deref());
    let mut session = parser.parse(source);
    let mut out = String::new();
    let mut error_diags: Vec<Diagnostic> = Vec::new();
    let mut count = 0;

    loop {
        match session.next() {
            ParseOutcome::Ok(stmt) => {
                if count > 0 {
                    out.push_str("----\n");
                }
                stmt.dump(&mut out, 0);
                count += 1;
            }
            ParseOutcome::Err(err) => {
                let start = err.offset().unwrap_or(0);
                let end = start + err.length().unwrap_or(0);
                error_diags.push(Diagnostic {
                    start_offset: start,
                    end_offset: end,
                    message: DiagnosticMessage::Other(err.message().to_string()),
                    severity: Severity::Error,
                    help: None,
                });
            }
            ParseOutcome::Done => break,
        }
    }

    print!("{out}");
    if error_diags.is_empty() {
        Ok(())
    } else {
        let n = error_diags.len();
        DiagnosticRenderer::new(source, file)
            .render_diagnostics(&error_diags, &mut io::stderr())
            .ok();
        Err(format!("{n} syntax error(s)"))
    }
}

fn cmd_fmt(
    dialect: Dialect,
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
            let out = format_source(&dialect, source, &config).map_err(|e| format!("{e}"))?;
            print!("{out}");
            Ok(())
        },
        |source, path, multi| {
            match format_source(&dialect, source, &config) {
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

fn format_source(
    dialect: &Dialect,
    source: &str,
    config: &FormatConfig,
) -> Result<String, FormatError> {
    Formatter::with_dialect_config(dialect.clone(), config).format(source)
}

fn cmd_validate(
    dialect: Dialect,
    files: Vec<String>,
    lang: Option<HostLanguage>,
) -> Result<(), String> {
    let config = ValidationConfig::default();
    let mut any_errors = false;

    let validate = |source: &str, file: &str| -> bool {
        match lang {
            Some(lang) => validate_embedded_source(&dialect, source, file, &config, lang),
            None => validate_source(&dialect, source, file, &config),
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

fn validate_source(dialect: &Dialect, source: &str, file: &str, config: &ValidationConfig) -> bool {
    let catalog = Catalog::new(dialect.clone());
    let mut analyzer = SemanticAnalyzer::with_dialect(dialect.clone());
    let model = analyzer.analyze(source, &catalog, config);
    DiagnosticRenderer::new(source, file)
        .render_diagnostics(model.diagnostics(), &mut io::stderr())
        .unwrap_or(false)
}

fn validate_embedded_source(
    dialect: &Dialect,
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

    let catalog = Catalog::new(dialect.clone());
    let diags = syntaqlite::embedded::EmbeddedAnalyzer::new(dialect.clone())
        .with_catalog(catalog)
        .with_config(*config)
        .validate(&fragments);

    DiagnosticRenderer::new(source, file)
        .render_diagnostics(&diags, &mut io::stderr())
        .unwrap_or(false)
}

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Runtime SQL commands (ast, fmt, lsp, validate) that require the `syntaqlite` crate.

use std::fs;
use std::io::{self, Read};
use std::ops::Deref;
use std::path::PathBuf;

use clap::ValueEnum;
use syntaqlite::any::AnyDialect;
use syntaqlite::any::{AnyParser, ParseOutcome};
use syntaqlite::util::DiagnosticRenderer;
use syntaqlite::{
    Catalog, Diagnostic, DiagnosticMessage, FormatConfig, FormatError, Formatter, KeywordCase,
    SemanticAnalyzer, Severity, ValidationConfig,
};

use super::{Cli, Command};

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum KeywordCasing {
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

fn require_dialect(dialect: Option<AnyDialect>) -> Result<AnyDialect, String> {
    dialect.ok_or_else(|| {
        "this command requires a dialect; build with --features=builtin-sqlite or use --dialect"
            .to_string()
    })
}

pub(crate) fn dispatch(cli: Cli, dialect: Option<AnyDialect>) -> Result<(), String> {
    let base = if let Some(path) = &cli.dialect_path {
        Some(
            AnyDialect::load(path, cli.dialect_name.as_deref()).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                std::process::exit(1);
            }),
        )
    } else {
        dialect
    };

    let configured = match base {
        Some(d) => Some(apply_version_cflags(
            d,
            cli.sqlite_version.as_ref(),
            &cli.sqlite_cflag,
        )?),
        None => None,
    };

    dispatch_commands(cli.command, configured)
}

fn apply_version_cflags(
    mut dialect: AnyDialect,
    version: Option<&String>,
    cflags: &[String],
) -> Result<AnyDialect, String> {
    use syntaqlite::util::{SqliteFlags, SqliteVersion};

    if let Some(v) = version {
        let ver = SqliteVersion::parse_with_latest(v)
            .map_err(|e| format!("invalid --sqlite-version: {e}"))?;
        dialect = dialect.with_version(ver);
    }

    if !cflags.is_empty() {
        let mut flags = SqliteFlags::default();
        for name in cflags {
            let flag = syntaqlite::util::SqliteFlag::from_name(name)
                .ok_or_else(|| format!("unknown --sqlite-cflag: {name}"))?;
            flags = flags.with(flag);
        }
        dialect = dialect.with_cflags(flags);
    }

    Ok(dialect)
}

fn dispatch_commands(command: Command, dialect: Option<AnyDialect>) -> Result<(), String> {
    match command {
        Command::Parse { files, output } => {
            require_dialect(dialect).and_then(|d| cmd_parse(&d, &files, output))
        }
        Command::Validate { files, lang } => {
            require_dialect(dialect).and_then(|d| cmd_validate(&d, &files, lang))
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
            let config = FormatConfig::default()
                .with_line_width(line_width)
                .with_keyword_case(match keyword_case {
                    KeywordCasing::Upper => KeywordCase::Upper,
                    KeywordCasing::Lower => KeywordCase::Lower,
                })
                .with_semicolons(semicolons);
            require_dialect(dialect).and_then(|d| cmd_fmt(&d, &files, &config, in_place))
        }
        Command::Version => {
            println!("syntaqlite {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Dialect(args) => crate::codegen::dispatch_dialect(&args),
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
    files: &[String],
    on_stdin: impl FnOnce(&str) -> Result<(), String>,
    mut on_file: impl FnMut(&str, &PathBuf, bool) -> Result<(), String>,
) -> Result<(), String> {
    let paths = expand_paths(files)?;

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

fn cmd_parse(
    dialect: &AnyDialect,
    files: &[String],
    output: crate::ParseOutput,
) -> Result<(), String> {
    let mut total_stmts = 0u64;
    let mut total_errors = 0u64;

    let paths = expand_paths(files)?;

    if paths.is_empty() {
        let source = read_stdin()?;
        let (s, e) = cmd_parse_source(dialect, &source, "<stdin>", output);
        total_stmts += s;
        total_errors += e;
    } else {
        let multi = paths.len() > 1;
        for path in &paths {
            let source =
                fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let file = path.display().to_string();
            if multi && matches!(output, crate::ParseOutput::Ast) {
                println!("==> {file} <==");
            }
            let (s, e) = cmd_parse_source(dialect, &source, &file, output);
            total_stmts += s;
            total_errors += e;
        }
    }

    if matches!(output, crate::ParseOutput::Summary) {
        println!("{total_stmts} statements parsed, {total_errors} errors");
    }

    if total_errors > 0 {
        Err(format!("{total_errors} syntax error(s)"))
    } else {
        Ok(())
    }
}

/// Parse a single source. Returns `(stmt_count, error_count)`.
fn cmd_parse_source(
    dialect: &AnyDialect,
    source: &str,
    file: &str,
    output: crate::ParseOutput,
) -> (u64, u64) {
    let parser = AnyParser::new(dialect.deref().clone());
    let mut session = parser.parse(source);
    let mut ast_out = String::new();
    let mut error_diags: Vec<Diagnostic> = Vec::new();
    let mut count = 0u64;

    loop {
        match session.next() {
            ParseOutcome::Ok(stmt) => {
                if matches!(output, crate::ParseOutput::Ast) {
                    if count > 0 {
                        ast_out.push_str("----\n");
                    }
                    stmt.dump(&mut ast_out, 0);
                }
                count += 1;
            }
            ParseOutcome::Err(err) => {
                let start = err.offset().unwrap_or(0);
                let end = start + err.length().unwrap_or(0);
                error_diags.push(Diagnostic::new(
                    start,
                    end,
                    DiagnosticMessage::Other(err.message().to_string()),
                    Severity::Error,
                    None,
                ));
            }
            ParseOutcome::Done => break,
        }
    }

    if matches!(output, crate::ParseOutput::Ast) {
        print!("{ast_out}");
    }

    let n_err = error_diags.len() as u64;
    if !error_diags.is_empty() {
        DiagnosticRenderer::new(source, file)
            .render_diagnostics(&error_diags, &mut io::stderr())
            .ok();
    }
    (count, n_err)
}

fn cmd_fmt(
    dialect: &AnyDialect,
    files: &[String],
    config: &FormatConfig,
    in_place: bool,
) -> Result<(), String> {
    let mut errors = Vec::new();
    process_files(
        files,
        |source| {
            if in_place {
                return Err("--in-place requires file arguments".to_string());
            }
            let out = format_source(dialect, source, config).map_err(|e| {
                render_format_error(&e, source, "<stdin>");
                format!("<stdin>: {e}")
            })?;
            print!("{out}");
            Ok(())
        },
        |source, path, multi| {
            match format_source(dialect, source, config) {
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
                    let label = path.display().to_string();
                    render_format_error(&e, source, &label);
                    errors.push(format!("{label}: {e}"));
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

fn render_format_error(e: &FormatError, source: &str, file: &str) {
    let start = e.offset().unwrap_or(0);
    let end = start + e.length().unwrap_or(0);
    let diag = Diagnostic::new(
        start,
        end,
        DiagnosticMessage::Other(e.message().to_owned()),
        Severity::Error,
        None,
    );
    DiagnosticRenderer::new(source, file)
        .render_diagnostic(&diag, &mut io::stderr())
        .ok();
}

fn format_source(
    dialect: &AnyDialect,
    source: &str,
    config: &FormatConfig,
) -> Result<String, FormatError> {
    Formatter::with_dialect_config(dialect.clone(), config).format(source)
}

fn cmd_validate(
    dialect: &AnyDialect,
    files: &[String],
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
    dialect: &AnyDialect,
    source: &str,
    file: &str,
    config: &ValidationConfig,
) -> bool {
    let catalog = Catalog::new(dialect.clone());
    let mut analyzer = SemanticAnalyzer::with_dialect(dialect.clone());
    let model = analyzer.analyze(source, &catalog, config);
    DiagnosticRenderer::new(source, file)
        .render_diagnostics(model.diagnostics(), &mut io::stderr())
        .unwrap_or(false)
}

fn validate_embedded_source(
    dialect: &AnyDialect,
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

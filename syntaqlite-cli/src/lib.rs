// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use syntaqlite_runtime::Dialect;
use syntaqlite_runtime::dialect::ffi as dialect_ffi;
use syntaqlite_runtime::fmt::{FormatConfig, Formatter, KeywordCase};

#[derive(Parser)]
#[command(about = "SQL formatting and analysis tools")]
struct Cli {
    /// Path to a shared library (.so/.dylib/.dll) providing an extension dialect.
    #[arg(long)]
    extension_dialect: Option<String>,

    /// Dialect name for symbol lookup (required with --extension-dialect).
    /// The library must export `syntaqlite_<name>_dialect`.
    #[arg(long)]
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
        #[arg(long)]
        semicolons: bool,
    },
    /// Generate amalgamated dialect C sources for embedding.
    ///
    /// Base SQLite grammar and node files are embedded in the binary.
    /// When `--actions-dir` / `--nodes-dir` are provided, those extension
    /// files are merged with the base (same-name files replace the base).
    GenerateDialect {
        /// Dialect name (e.g. "sqlite").
        #[arg(long, required = true)]
        dialect: String,
        /// Directory containing .y grammar action files (extensions only; base is embedded).
        #[arg(long)]
        actions_dir: Option<String>,
        /// Directory containing .synq node definitions (extensions only; base is embedded).
        #[arg(long)]
        nodes_dir: Option<String>,
        /// Path to SQLite's tokenize.c.
        #[arg(long, required = true)]
        tokenize_c: String,
        /// Path to syntaqlite-runtime directory (for full amalgamation).
        #[arg(long, required = true)]
        runtime_dir: String,
        /// Output directory for amalgamated files.
        #[arg(long, required = true)]
        output_dir: String,
    },
    // Hidden subcommands for codegen subprocess support.
    // generate_parser() and generate_keyword_hash() spawn current_exe() with
    // these subcommands. They must be present in any binary that calls codegen.
    #[command(hide = true)]
    Lemon {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(hide = true)]
    Mkkeyword {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
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
    let mut parser = syntaqlite_runtime::Parser::new(dialect);
    let mut cursor = parser.parse(source);
    let mut buf = String::new();
    let mut count = 0;

    while let Some(result) = cursor.next_statement() {
        let root_id = result.map_err(|e| format!("parse error: {e}"))?;
        if count > 0 {
            buf.push_str("----\n");
        }
        cursor.dump_node(root_id, &mut buf, 0);
        count += 1;
    }

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
    let mut formatter = Formatter::with_config(dialect, config)
        .map_err(|e| format!("failed to load formatter: {e}"))?;

    let paths = expand_paths(&files)?;

    if paths.is_empty() {
        if in_place {
            return Err("--in-place requires file arguments".to_string());
        }
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .map_err(|e| format!("reading stdin: {e}"))?;
        let out = formatter.format(&source).map_err(|e| format!("{e}"))?;
        print!("{out}");
        return Ok(());
    }

    let mut errors = Vec::new();
    for path in &paths {
        let source = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        match formatter.format(&source) {
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

fn cmd_generate_dialect(
    dialect: &str,
    actions_dir: Option<&str>,
    nodes_dir: Option<&str>,
    tokenize_c: &str,
    runtime_dir: &str,
    output_dir: &str,
) -> Result<(), String> {
    use syntaqlite_codegen::amalgamate;
    use syntaqlite_codegen::base_files;

    // Run codegen into a temp directory.
    let temp_dir = tempfile::TempDir::new().map_err(|e| format!("creating temp directory: {e}"))?;
    let temp = temp_dir.path();
    let csrc = temp.join("csrc");
    let include = temp.join("include").join(format!("syntaqlite_{dialect}"));
    fs::create_dir_all(&csrc).map_err(|e| format!("creating csrc dir: {e}"))?;
    fs::create_dir_all(&include).map_err(|e| format!("creating include dir: {e}"))?;

    // Load extension files from user dirs (if provided).
    let ext_y = match actions_dir {
        Some(dir) => syntaqlite_codegen::read_named_files_from_dir(dir, "y")?,
        None => Vec::new(),
    };
    let ext_synq = match nodes_dir {
        Some(dir) => syntaqlite_codegen::read_named_files_from_dir(dir, "synq")?,
        None => Vec::new(),
    };

    // Merge base + extensions.
    let merged_y = base_files::merge_file_sets(base_files::base_y_files(), &ext_y);
    let merged_synq = base_files::merge_file_sets(base_files::base_synq_files(), &ext_synq);

    codegen_to_dir_with_base(
        dialect,
        &merged_y,
        &merged_synq,
        tokenize_c,
        &csrc,
        &include,
    )?;

    // Full amalgamation: runtime + dialect into one pair of files.
    let result = amalgamate::amalgamate_full(dialect, Path::new(runtime_dir), temp.as_ref())?;

    let out = Path::new(output_dir);
    fs::create_dir_all(out).map_err(|e| format!("creating output dir: {e}"))?;
    fs::write(out.join(format!("syntaqlite_{dialect}.h")), &result.header)
        .map_err(|e| format!("writing header: {e}"))?;
    fs::write(out.join(format!("syntaqlite_{dialect}.c")), &result.source)
        .map_err(|e| format!("writing source: {e}"))?;

    eprintln!("wrote {}/syntaqlite_{dialect}.{{h,c}}", out.display());
    Ok(())
}

/// Run the codegen pipeline from merged in-memory file sets.
fn codegen_to_dir_with_base(
    dialect: &str,
    y_files: &[(String, String)],
    synq_files: &[(String, String)],
    tokenize_c: &str,
    csrc_dir: &Path,
    include_dir: &Path,
) -> Result<(), String> {
    let dialect_spec = syntaqlite_codegen::DialectSpec::new(dialect);
    let parser_prefix = dialect_spec.parser_symbol_prefix();

    // Extract extra keywords from extension .y files (terminals not in
    // the base keyword table are added to the hash). Duplicates with
    // base keywords are silently skipped by mkkeywordhash.
    let ext_y_contents: Vec<&str> = y_files
        .iter()
        .filter(|(name, _)| {
            // Only scan extension files (not base files).
            !syntaqlite_codegen::base_files::base_y_files()
                .iter()
                .any(|(base_name, _)| *base_name == name.as_str())
        })
        .map(|(_, content)| content.as_str())
        .collect();
    let extra_keywords = syntaqlite_codegen::extract_terminals_from_y(&ext_y_contents);

    let request = syntaqlite_codegen::CodegenRequest {
        dialect: &dialect_spec,
        y_files,
        synq_files,
        tokenize_c_path: tokenize_c,
        extra_keywords: &extra_keywords,
        parser_symbol_prefix: Some(&parser_prefix),
        include_rust: false,
    };
    let artifacts = syntaqlite_codegen::generate_codegen_artifacts(&request)?;

    // Write token header.
    fs::write(
        include_dir.join(dialect_spec.tokens_header_name()),
        dialect_spec.guarded_tokens_header(&artifacts.parse_h),
    )
    .map_err(|e| format!("writing {dialect}_tokens.h: {e}"))?;

    // AST headers.
    fs::write(
        include_dir.join(dialect_spec.node_header_name()),
        artifacts.ast_nodes_h,
    )
    .map_err(|e| format!("writing {dialect}_node.h: {e}"))?;

    fs::write(csrc_dir.join("dialect_builder.h"), artifacts.ast_builder_h)
        .map_err(|e| format!("writing dialect_builder.h: {e}"))?;

    // Parse engine (raw Lemon output, compiled as part of dialect unit).
    fs::write(csrc_dir.join("sqlite_parse.c"), artifacts.parse_c)
        .map_err(|e| format!("writing sqlite_parse.c: {e}"))?;

    // Forward-declaration headers for parser and tokenizer.
    let parse_h = syntaqlite_codegen::ast_codegen::generate_parse_h(dialect);
    fs::write(csrc_dir.join("sqlite_parse.h"), parse_h)
        .map_err(|e| format!("writing sqlite_parse.h: {e}"))?;

    let tokenize_h = syntaqlite_codegen::ast_codegen::generate_tokenize_h(dialect);
    fs::write(csrc_dir.join("sqlite_tokenize.h"), tokenize_h)
        .map_err(|e| format!("writing sqlite_tokenize.h: {e}"))?;

    // Tokenizer + keywords.
    fs::write(csrc_dir.join("sqlite_tokenize.c"), artifacts.tokenize_c)
        .map_err(|e| format!("writing sqlite_tokenize.c: {e}"))?;
    fs::write(csrc_dir.join("sqlite_keyword.c"), artifacts.keyword_c)
        .map_err(|e| format!("writing sqlite_keyword.c: {e}"))?;

    // Metadata + formatter data.
    fs::write(csrc_dir.join("dialect_meta.h"), artifacts.dialect_meta_h)
        .map_err(|e| format!("writing dialect_meta.h: {e}"))?;

    fs::write(csrc_dir.join("dialect_fmt.h"), artifacts.dialect_fmt_h)
        .map_err(|e| format!("writing dialect_fmt.h: {e}"))?;

    // Dialect descriptor + public API.
    fs::write(csrc_dir.join("dialect.c"), artifacts.dialect_c)
        .map_err(|e| format!("writing dialect.c: {e}"))?;

    fs::write(
        include_dir.join(dialect_spec.dialect_header_name()),
        artifacts.dialect_h,
    )
    .map_err(|e| format!("writing {dialect}.h: {e}"))?;

    fs::write(
        csrc_dir.join(dialect_spec.dialect_dispatch_header_name()),
        artifacts.dialect_dispatch_h,
    )
    .map_err(|e| format!("writing {dialect}_dialect_dispatch.h: {e}"))?;

    Ok(())
}

/// Load an extension dialect from a shared library.
///
/// The library must export a function `syntaqlite_<name>_dialect` returning
/// `*const SyntaqliteDialect`. The returned `Library` must be kept alive for
/// the `Dialect` to remain valid.
unsafe fn load_extension_dialect(
    path: &str,
    name: &str,
) -> Result<(libloading::Library, Dialect<'static>), String> {
    let lib = unsafe {
        libloading::Library::new(path).map_err(|e| format!("failed to load {path}: {e}"))?
    };

    let symbol_name = format!("syntaqlite_{name}_dialect");
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
pub fn run(name: &str, dialect: &Dialect) {
    let cli =
        Cli::try_parse_from(std::iter::once(name.to_string()).chain(std::env::args().skip(1)))
            .unwrap_or_else(|e| e.exit());

    // Validate flag combination.
    if cli.extension_dialect.is_some() != cli.dialect_name.is_some() {
        eprintln!("error: --extension-dialect and --dialect-name must be used together");
        std::process::exit(1);
    }

    // Load extension dialect if requested. The library handle must stay alive
    // until after the command finishes.
    let _ext_lib;
    let active_dialect: &Dialect;
    let ext_dialect;

    if let (Some(path), Some(ext_name)) = (&cli.extension_dialect, &cli.dialect_name) {
        let (lib, d) = unsafe { load_extension_dialect(path, ext_name) }.unwrap_or_else(|e| {
            eprintln!("error: {e}");
            std::process::exit(1);
        });
        _ext_lib = Some(lib);
        ext_dialect = d;
        active_dialect = &ext_dialect;
    } else {
        _ext_lib = None;
        active_dialect = dialect;
    }

    let result = match cli.command {
        Command::Ast { files } => cmd_ast(active_dialect, files),
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
            cmd_fmt(active_dialect, files, config, in_place, semicolons)
        }
        Command::GenerateDialect {
            dialect: dialect_name,
            actions_dir,
            nodes_dir,
            tokenize_c,
            runtime_dir,
            output_dir,
        } => cmd_generate_dialect(
            &dialect_name,
            actions_dir.as_deref(),
            nodes_dir.as_deref(),
            &tokenize_c,
            &runtime_dir,
            &output_dir,
        ),
        Command::Lemon { args } => {
            syntaqlite_codegen::run_lemon(&args);
        }
        Command::Mkkeyword { args } => {
            syntaqlite_codegen::run_mkkeyword(&args);
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

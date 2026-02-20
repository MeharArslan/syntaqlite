// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use syntaqlite_runtime::Dialect;
use syntaqlite_runtime::fmt::{FormatConfig, Formatter, KeywordCase};

#[derive(Parser)]
#[command(about = "SQL formatting and analysis tools")]
struct Cli {
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
        Some(dir) => read_files_from_dir(dir, "y")?,
        None => Vec::new(),
    };
    let ext_synq = match nodes_dir {
        Some(dir) => read_files_from_dir(dir, "synq")?,
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

/// Read all files with the given extension from a directory.
fn read_files_from_dir(dir: &str, ext: &str) -> Result<Vec<(String, String)>, String> {
    let dir_path = Path::new(dir);
    if !dir_path.is_dir() {
        return Err(format!("{dir} is not a directory"));
    }
    let mut files: Vec<(String, String)> = fs::read_dir(dir_path)
        .map_err(|e| format!("reading {dir}: {e}"))?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension()?.to_str()? == ext {
                let name = path.file_name()?.to_str()?.to_string();
                let content = fs::read_to_string(&path).ok()?;
                Some((name, content))
            } else {
                None
            }
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
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
    // Parse node definitions from in-memory contents.
    let mut all_items = Vec::new();
    for (name, content) in synq_files {
        let items = syntaqlite_codegen::node_parser::parse_node(content)
            .map_err(|e| format!("{name}: {e}"))?;
        all_items.extend(items);
    }

    // Generate parser (lemon) from in-memory .y contents.
    let work_dir = tempfile::TempDir::new().map_err(|e| format!("creating work directory: {e}"))?;
    syntaqlite_codegen::generate_parser_from_contents(y_files, work_dir.path().to_str().unwrap())?;

    // Extract tokenizer.
    let (tokenize_content, extract_result) =
        syntaqlite_codegen::extract_tokenizer(tokenize_c, dialect)?;

    // Generate keyword hash.
    let keyword_c = syntaqlite_codegen::generate_keyword_hash(&extract_result, dialect)?;

    // Write token header.
    let tokens_content = fs::read_to_string(work_dir.path().join("parse.h"))
        .map_err(|e| format!("reading parse.h: {e}"))?;
    let upper = dialect.to_uppercase();
    let guard = format!("SYNTAQLITE_{upper}_TOKENS_H");
    let guarded = format!(
        "/*\n\
         ** The author disclaims copyright to this source code.  In place of\n\
         ** a legal notice, here is a blessing:\n\
         **\n\
         **    May you do good and not evil.\n\
         **    May you find forgiveness for yourself and forgive others.\n\
         **    May you share freely, never taking more than you give.\n\
         **\n\
         ** @generated by syntaqlite-codegen — DO NOT EDIT\n\
         */\n\
         \n\
         #ifndef {guard}\n\
         #define {guard}\n\
         \n\
         {tokens_content}\
         \n\
         #endif  // {guard}\n",
    );
    fs::write(include_dir.join(format!("{dialect}_tokens.h")), guarded)
        .map_err(|e| format!("writing {dialect}_tokens.h: {e}"))?;

    // AST headers.
    let ast_nodes_h = syntaqlite_codegen::ast_codegen::generate_ast_nodes_h(&all_items, dialect);
    fs::write(include_dir.join(format!("{dialect}_node.h")), ast_nodes_h)
        .map_err(|e| format!("writing {dialect}_node.h: {e}"))?;

    let ast_builder_h =
        syntaqlite_codegen::ast_codegen::generate_ast_builder_h(&all_items, dialect);
    fs::write(csrc_dir.join("dialect_builder.h"), ast_builder_h)
        .map_err(|e| format!("writing dialect_builder.h: {e}"))?;

    // Parse engine (raw Lemon output, compiled as part of dialect unit).
    let raw_parse_c = fs::read_to_string(work_dir.path().join("parse.c"))
        .map_err(|e| format!("reading parse.c: {e}"))?;
    fs::write(csrc_dir.join("sqlite_parse.c"), raw_parse_c)
        .map_err(|e| format!("writing sqlite_parse.c: {e}"))?;

    // Tokenizer + keywords.
    fs::write(csrc_dir.join("sqlite_tokenize.c"), tokenize_content)
        .map_err(|e| format!("writing sqlite_tokenize.c: {e}"))?;
    fs::write(csrc_dir.join("sqlite_keyword.c"), keyword_c)
        .map_err(|e| format!("writing sqlite_keyword.c: {e}"))?;

    // Metadata + formatter data.
    let ast_meta_h = syntaqlite_codegen::ast_codegen::generate_c_field_meta(&all_items, dialect);
    fs::write(csrc_dir.join("dialect_meta.h"), ast_meta_h)
        .map_err(|e| format!("writing dialect_meta.h: {e}"))?;

    let fmt_data_h = syntaqlite_codegen::ast_codegen::generate_c_fmt_arrays(&all_items);
    fs::write(csrc_dir.join("dialect_fmt.h"), fmt_data_h)
        .map_err(|e| format!("writing dialect_fmt.h: {e}"))?;

    // Dialect descriptor + public API.
    let dialect_c = syntaqlite_codegen::ast_codegen::generate_dialect_c(dialect);
    fs::write(csrc_dir.join("dialect.c"), dialect_c)
        .map_err(|e| format!("writing dialect.c: {e}"))?;

    let dialect_h = syntaqlite_codegen::ast_codegen::generate_dialect_h(dialect);
    fs::write(include_dir.join(format!("{dialect}.h")), dialect_h)
        .map_err(|e| format!("writing {dialect}.h: {e}"))?;

    let dialect_dispatch_h =
        syntaqlite_codegen::ast_codegen::generate_dialect_dispatch_h(dialect);
    fs::write(
        csrc_dir.join(format!("{dialect}_dialect_dispatch.h")),
        dialect_dispatch_h,
    )
    .map_err(|e| format!("writing {dialect}_dialect_dispatch.h: {e}"))?;

    Ok(())
}

/// Run the CLI with the given dialect configuration.
pub fn run(name: &str, dialect: &Dialect) {
    let cli =
        Cli::try_parse_from(std::iter::once(name.to_string()).chain(std::env::args().skip(1)))
            .unwrap_or_else(|e| e.exit());

    let result = match cli.command {
        Command::Ast { files } => cmd_ast(dialect, files),
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
            cmd_fmt(dialect, files, config, in_place, semicolons)
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
            syntaqlite_codegen::lemon::run_lemon(&args);
        }
        Command::Mkkeyword { args } => {
            syntaqlite_codegen::mkkeyword::run_mkkeyword(&args);
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

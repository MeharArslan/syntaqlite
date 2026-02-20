// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use syntaqlite_runtime::fmt::{FormatConfig, Formatter, KeywordCase};
use syntaqlite_runtime::Dialect;

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
    GenerateDialect {
        /// Dialect name (e.g. "sqlite").
        #[arg(long, required = true)]
        dialect: String,
        /// Directory containing .y grammar action files.
        #[arg(long, required = true)]
        actions_dir: String,
        /// Directory containing .synq node definitions.
        #[arg(long, required = true)]
        nodes_dir: String,
        /// Path to SQLite's tokenize.c.
        #[arg(long, required = true)]
        tokenize_c: String,
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
        let source =
            fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
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
        let source =
            fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        match formatter.format(&source) {
            Ok(out) => {
                if in_place {
                    if out != source {
                        fs::write(path, &out)
                            .map_err(|e| format!("{}: {e}", path.display()))?;
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
    actions_dir: &str,
    nodes_dir: &str,
    tokenize_c: &str,
    output_dir: &str,
) -> Result<(), String> {
    use syntaqlite_codegen::amalgamate;

    // Run codegen into a temp directory.
    let temp_dir = tempfile::TempDir::new()
        .map_err(|e| format!("creating temp directory: {e}"))?;
    let temp = temp_dir.path();
    let csrc = temp.join("csrc");
    let include = temp.join("include").join("syntaqlite");
    fs::create_dir_all(&csrc).map_err(|e| format!("creating csrc dir: {e}"))?;
    fs::create_dir_all(&include).map_err(|e| format!("creating include dir: {e}"))?;

    codegen_to_dir(actions_dir, nodes_dir, tokenize_c, &csrc, &include)?;

    // Amalgamate the generated files.
    let result = amalgamate::amalgamate_dialect(dialect, temp.as_ref())?;

    let out = Path::new(output_dir);
    fs::create_dir_all(out).map_err(|e| format!("creating output dir: {e}"))?;
    fs::write(out.join(format!("syntaqlite_{dialect}.h")), &result.header)
        .map_err(|e| format!("writing header: {e}"))?;
    fs::write(out.join(format!("syntaqlite_{dialect}.c")), &result.source)
        .map_err(|e| format!("writing source: {e}"))?;

    eprintln!("wrote {}/syntaqlite_{dialect}.{{h,c}}", out.display());
    Ok(())
}

/// Run the codegen pipeline, writing C outputs to `csrc_dir` and `include_dir`.
fn codegen_to_dir(
    actions_dir: &str,
    nodes_dir: &str,
    tokenize_c: &str,
    csrc_dir: &Path,
    include_dir: &Path,
) -> Result<(), String> {
    // Parse node definitions.
    let nodes_path = Path::new(nodes_dir);
    let mut synq_files: Vec<_> = fs::read_dir(nodes_path)
        .map_err(|e| format!("reading {nodes_dir}: {e}"))?
        .filter_map(|e| {
            let p = e.ok()?.path();
            (p.extension()?.to_str()? == "synq").then_some(p)
        })
        .collect();
    synq_files.sort();
    let mut all_items = Vec::new();
    for path in &synq_files {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("?");
        let content = fs::read_to_string(path)
            .map_err(|e| format!("{name}: {e}"))?;
        let items = syntaqlite_codegen::node_parser::parse_node(&content)
            .map_err(|e| format!("{name}: {e}"))?;
        all_items.extend(items);
    }

    // Generate parser (lemon).
    let work_dir = tempfile::TempDir::new()
        .map_err(|e| format!("creating work directory: {e}"))?;
    syntaqlite_codegen::generate_parser(actions_dir, work_dir.path().to_str().unwrap())?;

    // Extract tokenizer.
    let (tokenize_content, extract_result) =
        syntaqlite_codegen::extract_tokenizer(tokenize_c)?;

    // Generate keyword hash.
    let (keyword_tables, keyword_func) =
        syntaqlite_codegen::generate_keyword_hash(&extract_result)?;

    // Write token header.
    let tokens_content = fs::read_to_string(work_dir.path().join("parse.h"))
        .map_err(|e| format!("reading parse.h: {e}"))?;
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
         #ifndef SYNTAQLITE_SQLITE_TOKENS_H\n\
         #define SYNTAQLITE_SQLITE_TOKENS_H\n\
         \n\
         {}\
         \n\
         #endif  // SYNTAQLITE_SQLITE_TOKENS_H\n",
        tokens_content,
    );
    fs::write(include_dir.join("sqlite_tokens.h"), guarded)
        .map_err(|e| format!("writing sqlite_tokens.h: {e}"))?;

    // AST headers.
    let ast_nodes_h = syntaqlite_codegen::ast_codegen::generate_ast_nodes_h(&all_items);
    fs::write(include_dir.join("sqlite_node.h"), ast_nodes_h)
        .map_err(|e| format!("writing sqlite_node.h: {e}"))?;

    let ast_builder_h = syntaqlite_codegen::ast_codegen::generate_ast_builder_h(&all_items);
    fs::write(csrc_dir.join("dialect_builder.h"), ast_builder_h)
        .map_err(|e| format!("writing dialect_builder.h: {e}"))?;

    // Parse engine + data header.
    let raw_parse_c = fs::read_to_string(work_dir.path().join("parse.c"))
        .map_err(|e| format!("reading parse.c: {e}"))?;
    let (parse_c, parse_data_h) = syntaqlite_codegen::split_parse_c(&raw_parse_c)?;
    fs::write(csrc_dir.join("sqlite_parse.c"), parse_c)
        .map_err(|e| format!("writing sqlite_parse.c: {e}"))?;
    fs::write(csrc_dir.join("dialect_parse.h"), parse_data_h)
        .map_err(|e| format!("writing dialect_parse.h: {e}"))?;

    // Tokenizer + keywords.
    fs::write(csrc_dir.join("sqlite_tokenize.c"), tokenize_content)
        .map_err(|e| format!("writing sqlite_tokenize.c: {e}"))?;
    fs::write(csrc_dir.join("sqlite_keyword_tables.h"), keyword_tables)
        .map_err(|e| format!("writing sqlite_keyword_tables.h: {e}"))?;
    fs::write(csrc_dir.join("sqlite_keyword.c"), keyword_func)
        .map_err(|e| format!("writing sqlite_keyword.c: {e}"))?;

    // Metadata + formatter data.
    let ast_meta_h = syntaqlite_codegen::ast_codegen::generate_c_field_meta(&all_items);
    fs::write(csrc_dir.join("dialect_meta.h"), ast_meta_h)
        .map_err(|e| format!("writing dialect_meta.h: {e}"))?;

    let fmt_data_h = syntaqlite_codegen::ast_codegen::generate_c_fmt_arrays(&all_items);
    fs::write(csrc_dir.join("dialect_fmt.h"), fmt_data_h)
        .map_err(|e| format!("writing dialect_fmt.h: {e}"))?;

    // Grammar types header.
    let grammar_types_h = syntaqlite_codegen::ast_codegen::generate_grammar_types_h(&all_items);
    fs::write(csrc_dir.join("dialect_grammar_types.h"), grammar_types_h)
        .map_err(|e| format!("writing dialect_grammar_types.h: {e}"))?;

    Ok(())
}

/// Run the CLI with the given dialect configuration.
pub fn run(name: &str, dialect: &Dialect) {
    let cli = Cli::try_parse_from(
        std::iter::once(name.to_string()).chain(std::env::args().skip(1)),
    )
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
            output_dir,
        } => cmd_generate_dialect(&dialect_name, &actions_dir, &nodes_dir, &tokenize_c, &output_dir),
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

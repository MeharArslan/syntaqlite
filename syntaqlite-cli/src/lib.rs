// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fs;
use std::io::{self, Read};
#[cfg(feature = "codegen")]
use std::path::Path;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use syntaqlite_runtime::dialect::ffi as dialect_ffi;
use syntaqlite_runtime::fmt::{FormatConfig, Formatter, KeywordCase};
use syntaqlite_runtime::{Dialect, ParseError, Parser as RuntimeParser};

mod lsp;

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
    /// Generate amalgamated dialect C sources for embedding.
    ///
    /// Base SQLite grammar and node files are embedded in the binary.
    /// When `--actions-dir` / `--nodes-dir` are provided, those extension
    /// files are merged with the base (same-name files replace the base).
    #[cfg(feature = "codegen")]
    Dialect {
        /// Dialect identifier (e.g. "sqlite").
        #[arg(long, required = true)]
        name: String,
        /// Directory containing .y grammar action files (extensions only; base is embedded).
        #[arg(long)]
        actions_dir: Option<String>,
        /// Directory containing .synq node definitions (extensions only; base is embedded).
        #[arg(long)]
        nodes_dir: Option<String>,
        #[command(subcommand)]
        command: DialectCommand,
    },
    /// Run the full codegen pipeline (grammar extraction, parser generation,
    /// tokenizer extraction, keyword hash, AST metadata, Rust bindings).
    #[cfg(feature = "codegen")]
    Codegen {
        #[arg(long, required = true)]
        actions_dir: String,
        #[arg(long, required = true)]
        nodes_dir: String,
        #[arg(long, default_value = "syntaqlite/csrc")]
        output_dir: String,
    },
    /// Produce C amalgamation files (single-file compilation units).
    #[cfg(feature = "codegen")]
    Amalgamate {
        /// Dialect name (e.g. "sqlite").
        #[arg(long, required = true)]
        dialect: String,
        /// Path to the syntaqlite-runtime crate root.
        #[arg(long, required = true)]
        runtime_dir: String,
        /// Path to the dialect crate root (e.g. syntaqlite/).
        #[arg(long, required = true)]
        dialect_dir: String,
        /// Output directory for generated files.
        #[arg(long, required = true)]
        output_dir: String,
        /// Emit only the runtime amalgamation.
        #[arg(long)]
        runtime_only: bool,
        /// Emit only the dialect amalgamation (references runtime header).
        #[arg(long)]
        dialect_only: bool,
    },
    // Hidden subcommands for codegen subprocess support.
    // generate_parser() and generate_keyword_hash() spawn current_exe() with
    // these subcommands. They must be present in any binary that calls codegen.
    #[cfg(feature = "codegen")]
    #[command(hide = true)]
    Lemon {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[cfg(feature = "codegen")]
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

#[cfg(feature = "codegen")]
#[derive(Subcommand)]
enum DialectCommand {
    /// Emit amalgamated C/H files.
    Csrc {
        /// Output directory for amalgamated files.
        #[arg(long, required = true)]
        output_dir: String,
    },
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

#[cfg(feature = "codegen")]
fn cmd_generate_dialect(
    dialect: &str,
    actions_dir: Option<&str>,
    nodes_dir: Option<&str>,
    output_dir: &str,
) -> Result<(), String> {
    use syntaqlite_codegen::amalgamate;
    use syntaqlite_codegen_sqlite::base_files;

    // Run codegen into a temp directory.
    let temp_dir = tempfile::TempDir::new().map_err(|e| format!("creating temp directory: {e}"))?;
    let temp = temp_dir.path();
    let csrc = temp.join("csrc");
    let include = temp.join("include").join(format!("syntaqlite_{dialect}"));
    fs::create_dir_all(&csrc).map_err(|e| format!("creating csrc dir: {e}"))?;
    fs::create_dir_all(&include).map_err(|e| format!("creating include dir: {e}"))?;

    // Load extension files from user dirs (if provided).
    let ext_y = match actions_dir {
        Some(dir) => syntaqlite_codegen_sqlite::read_named_files_from_dir(dir, "y")?,
        None => Vec::new(),
    };
    let ext_synq = match nodes_dir {
        Some(dir) => syntaqlite_codegen_sqlite::read_named_files_from_dir(dir, "synq")?,
        None => Vec::new(),
    };

    // Merge base + extensions.
    let merged_y = base_files::merge_file_sets(base_files::base_y_files(), &ext_y);
    let merged_synq = base_files::merge_file_sets(base_files::base_synq_files(), &ext_synq);

    codegen_to_dir_with_base(dialect, &merged_y, &merged_synq, &csrc, &include)?;

    let out = Path::new(output_dir);
    fs::create_dir_all(out).map_err(|e| format!("creating output dir: {e}"))?;
    let result = amalgamate::amalgamate_dialect(dialect, temp.as_ref())?;
    fs::write(out.join(format!("syntaqlite_{dialect}.h")), &result.header)
        .map_err(|e| format!("writing header: {e}"))?;
    fs::write(out.join(format!("syntaqlite_{dialect}.c")), &result.source)
        .map_err(|e| format!("writing source: {e}"))?;
    eprintln!("wrote {}/syntaqlite_{dialect}.{{h,c}}", out.display());
    Ok(())
}

/// Run the codegen pipeline from merged in-memory file sets.
#[cfg(feature = "codegen")]
fn codegen_to_dir_with_base(
    dialect: &str,
    y_files: &[(String, String)],
    synq_files: &[(String, String)],
    csrc_dir: &Path,
    include_dir: &Path,
) -> Result<(), String> {
    let dialect_spec = syntaqlite_codegen_sqlite::DialectNaming::new(dialect);
    let parser_prefix = dialect_spec.parser_symbol_prefix();

    // Extract extra keywords from extension .y files (terminals not in
    // the base keyword table are added to the hash). Duplicates with
    // base keywords are silently skipped by mkkeywordhash.
    let ext_y_contents: Vec<&str> = y_files
        .iter()
        .filter(|(name, _)| {
            // Only scan extension files (not base files).
            !syntaqlite_codegen_sqlite::base_files::base_y_files()
                .iter()
                .any(|(base_name, _)| *base_name == name.as_str())
        })
        .map(|(_, content)| content.as_str())
        .collect();
    let extra_keywords = syntaqlite_codegen_sqlite::extract_terminals_from_y(&ext_y_contents);

    let request = syntaqlite_codegen_sqlite::CodegenRequest {
        dialect: &dialect_spec,
        y_files,
        synq_files,
        extra_keywords: &extra_keywords,
        parser_symbol_prefix: Some(&parser_prefix),
        include_rust: false,
    };
    let artifacts = syntaqlite_codegen_sqlite::generate_codegen_artifacts(&request)?;

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
    let parse_h =
        syntaqlite_codegen::dialect_codegen::generate_parse_h(dialect);
    fs::write(csrc_dir.join("sqlite_parse.h"), parse_h)
        .map_err(|e| format!("writing sqlite_parse.h: {e}"))?;

    let tokenize_h =
        syntaqlite_codegen::dialect_codegen::generate_tokenize_h(dialect);
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

    fs::write(
        csrc_dir.join("dialect_tokens.h"),
        artifacts.dialect_tokens_h,
    )
    .map_err(|e| format!("writing dialect_tokens.h: {e}"))?;

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

/// Delete any .c/.h files in `dir` whose first 512 bytes contain the autogenerated marker.
#[cfg(feature = "codegen")]
fn clean_generated_files(dir: &Path, verbose: bool) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "c" && ext != "h" {
            continue;
        }
        // Read just the first 512 bytes to check for the marker
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let prefix = &content[..content.len().min(512)];
        if prefix.contains(syntaqlite_codegen::AUTOGENERATED_MARKER) {
            if verbose {
                eprintln!("  Removing stale generated file: {}", path.display());
            }
            let _ = fs::remove_file(&path);
        }
    }
}

#[cfg(feature = "codegen")]
fn log_verbose(verbose: bool, message: &str) {
    if verbose {
        eprintln!("{message}");
    }
}

#[cfg(feature = "codegen")]
fn ensure_dir(path: &Path, label: &str) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("Failed to create {label}: {e}"))
}

#[cfg(feature = "codegen")]
fn write_file(path: &Path, content: impl AsRef<[u8]>) -> Result<(), String> {
    fs::write(path, content).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

#[cfg(feature = "codegen")]
fn handle_codegen(
    actions_dir: &str,
    nodes_dir: &str,
    output_dir: &str,
    verbose: bool,
) -> Result<(), String> {
    let dialect = syntaqlite_codegen_sqlite::DialectNaming::new("sqlite");

    log_verbose(verbose, "Loading grammar + node definition files...");
    let y_files = syntaqlite_codegen_sqlite::read_named_files_from_dir(actions_dir, "y")?;
    let synq_files = syntaqlite_codegen_sqlite::read_named_files_from_dir(nodes_dir, "synq")?;

    log_verbose(verbose, "Running unified codegen pipeline...");
    let no_keywords: Vec<String> = Vec::new();
    let request = syntaqlite_codegen_sqlite::CodegenRequest {
        dialect: &dialect,
        y_files: &y_files,
        synq_files: &synq_files,
        extra_keywords: &no_keywords,
        parser_symbol_prefix: None,
        include_rust: true,
    };
    let artifacts = syntaqlite_codegen_sqlite::generate_codegen_artifacts(&request)?;
    let outputs = syntaqlite_codegen_sqlite::sqlite_output_manifest(&dialect, artifacts)?;

    // Step 4: Clean stale generated files, then write outputs
    let out = Path::new(output_dir);
    let include_dir = Path::new(output_dir)
        .parent()
        .unwrap_or(Path::new("."))
        .join(format!("include/{}", dialect.include_dir_name()));
    let rust_src_dir = Path::new(output_dir)
        .parent()
        .unwrap_or(Path::new("."))
        .join("src");

    for dir in [out, include_dir.as_path()] {
        if dir.is_dir() {
            clean_generated_files(dir, verbose);
        }
    }

    ensure_dir(out, "output directory")?;
    ensure_dir(&include_dir, "include directory")?;
    ensure_dir(&rust_src_dir, "Rust src directory")?;

    log_verbose(verbose, "Writing output files...");
    for output in outputs {
        let dir = match output.bucket {
            syntaqlite_codegen_sqlite::OutputBucket::Include => &include_dir,
            syntaqlite_codegen_sqlite::OutputBucket::DialectCsrc => out,
            syntaqlite_codegen_sqlite::OutputBucket::RustSrc => &rust_src_dir,
        };
        write_file(&dir.join(output.file_name), output.content)?;
    }

    log_verbose(verbose, "Code generation complete");
    Ok(())
}

#[cfg(feature = "codegen")]
fn write_amalgamation_outputs(
    out: &Path,
    header_name: &str,
    source_name: &str,
    result: &syntaqlite_codegen::amalgamate::AmalgamateOutput,
) -> Result<(), String> {
    write_file(&out.join(header_name), &result.header)?;
    write_file(&out.join(source_name), &result.source)?;
    if let Some(ext) = &result.ext_header {
        write_file(&out.join("syntaqlite_ext.h"), ext)?;
    }
    Ok(())
}

#[cfg(feature = "codegen")]
fn handle_amalgamate(
    dialect: &str,
    runtime_dir: &str,
    dialect_dir: &str,
    output_dir: &str,
    runtime_only: bool,
    dialect_only: bool,
    verbose: bool,
) -> Result<(), String> {
    use syntaqlite_codegen::amalgamate;

    let out = Path::new(output_dir);
    ensure_dir(out, "output directory")?;

    let runtime = PathBuf::from(runtime_dir);
    let dialect_path = PathBuf::from(dialect_dir);

    if runtime_only {
        log_verbose(verbose, "Generating runtime amalgamation...");
        let result = amalgamate::amalgamate_runtime(&runtime)?;
        write_amalgamation_outputs(out, "syntaqlite_runtime.h", "syntaqlite_runtime.c", &result)?;
        log_verbose(verbose, "Wrote syntaqlite_runtime.{h,c} + syntaqlite_ext.h");
    } else {
        let (phase, result) = if dialect_only {
            (
                "Generating dialect amalgamation...",
                amalgamate::amalgamate_dialect(dialect, &dialect_path)?,
            )
        } else {
            (
                "Generating full amalgamation...",
                amalgamate::amalgamate_full(dialect, &runtime, &dialect_path)?,
            )
        };
        log_verbose(verbose, phase);
        let base = format!("syntaqlite_{dialect}");
        write_amalgamation_outputs(out, &format!("{base}.h"), &format!("{base}.c"), &result)?;
        log_verbose(verbose, &format!("Wrote {base}.{{h,c}}"));
    }

    Ok(())
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
pub fn run(name: &str, dialect: &Dialect) {
    let cli =
        Cli::try_parse_from(std::iter::once(name.to_string()).chain(std::env::args().skip(1)))
            .unwrap_or_else(|e| e.exit());

    // Load a dynamic dialect if requested. The library handle must stay alive
    // until after the command finishes.
    let _dialect_lib;
    let dyn_dialect;
    let active_dialect: &Dialect;

    if let Some(path) = &cli.dialect_path {
        let (lib, d) = unsafe { load_dynamic_dialect(path, cli.dialect_name.as_deref()) }
            .unwrap_or_else(|e| {
                eprintln!("error: {e}");
                std::process::exit(1);
            });
        _dialect_lib = Some(lib);
        dyn_dialect = d;
        active_dialect = &dyn_dialect;
    } else {
        _dialect_lib = None;
        active_dialect = dialect;
    }

    // The `verbose` flag is not part of the shared Cli struct, but codegen
    // subcommands accept it via their own global flag. For the absorbed
    // Codegen/Amalgamate commands we hard-code verbose=false; the old
    // syntaqlite-codegen binary had a top-level --verbose flag, but we
    // keep things simple here.

    let result = match cli.command {
        Command::Ast { files } => cmd_ast(active_dialect, files),
        Command::Lsp => lsp::cmd_lsp(active_dialect),
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
        #[cfg(feature = "codegen")]
        Command::Dialect {
            name,
            actions_dir,
            nodes_dir,
            command,
        } => match command {
            DialectCommand::Csrc { output_dir } => cmd_generate_dialect(
                &name,
                actions_dir.as_deref(),
                nodes_dir.as_deref(),
                &output_dir,
            ),
        },
        #[cfg(feature = "codegen")]
        Command::Codegen {
            actions_dir,
            nodes_dir,
            output_dir,
        } => handle_codegen(&actions_dir, &nodes_dir, &output_dir, false),
        #[cfg(feature = "codegen")]
        Command::Amalgamate {
            dialect,
            runtime_dir,
            dialect_dir,
            output_dir,
            runtime_only,
            dialect_only,
        } => handle_amalgamate(
            &dialect,
            &runtime_dir,
            &dialect_dir,
            &output_dir,
            runtime_only,
            dialect_only,
            false,
        ),
        #[cfg(feature = "codegen")]
        Command::Lemon { args } => {
            syntaqlite_codegen_sqlite::run_lemon(&args);
        }
        #[cfg(feature = "codegen")]
        Command::Mkkeyword { args } => {
            syntaqlite_codegen_sqlite::run_mkkeyword(&args);
        }
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

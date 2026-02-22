// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;

/// Codegen-related CLI subcommands.
///
/// Flattened into the top-level `Command` enum so the CLI interface is unchanged.
#[derive(Subcommand)]
pub(crate) enum CodegenCommand {
    /// Generate amalgamated dialect C sources for embedding.
    ///
    /// Base SQLite grammar and node files are embedded in the binary.
    /// When `--actions-dir` / `--nodes-dir` are provided, those extension
    /// files are merged with the base (same-name files replace the base).
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
    Codegen {
        #[arg(long, required = true)]
        actions_dir: String,
        #[arg(long, required = true)]
        nodes_dir: String,
        #[arg(long, default_value = "syntaqlite/csrc")]
        output_dir: String,
    },
    /// Produce C amalgamation files (single-file compilation units).
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

#[derive(Subcommand)]
pub(crate) enum DialectCommand {
    /// Emit amalgamated C/H files.
    Csrc {
        /// Output directory for amalgamated files.
        #[arg(long, required = true)]
        output_dir: String,
    },
}

/// Dispatch a codegen subcommand. Called from `run()` in lib.rs.
pub(crate) fn dispatch(command: CodegenCommand) -> Result<(), String> {
    match command {
        CodegenCommand::Dialect {
            name,
            actions_dir,
            nodes_dir,
            command,
        } => match command {
            DialectCommand::Csrc { output_dir } => {
                cmd_generate_dialect(&name, actions_dir.as_deref(), nodes_dir.as_deref(), &output_dir)
            }
        },
        CodegenCommand::Codegen {
            actions_dir,
            nodes_dir,
            output_dir,
        } => handle_codegen(&actions_dir, &nodes_dir, &output_dir, false),
        CodegenCommand::Amalgamate {
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
        CodegenCommand::Lemon { args } => syntaqlite_codegen_sqlite::run_lemon(&args),
        CodegenCommand::Mkkeyword { args } => syntaqlite_codegen_sqlite::run_mkkeyword(&args),
    }
}

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
    let parse_h = syntaqlite_codegen::dialect_codegen::generate_parse_h(dialect);
    fs::write(csrc_dir.join("sqlite_parse.h"), parse_h)
        .map_err(|e| format!("writing sqlite_parse.h: {e}"))?;

    let tokenize_h = syntaqlite_codegen::dialect_codegen::generate_tokenize_h(dialect);
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

fn log_verbose(verbose: bool, message: &str) {
    if verbose {
        eprintln!("{message}");
    }
}

fn ensure_dir(path: &Path, label: &str) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("Failed to create {label}: {e}"))
}

fn write_file(path: &Path, content: impl AsRef<[u8]>) -> Result<(), String> {
    fs::write(path, content).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

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

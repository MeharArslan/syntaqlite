// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fs;
use std::path::Path;

use clap::Parser;

use crate::fs_util::{ensure_dir, write_file};

// Hardcoded workspace paths for --output-type=sqlite
const SQLITE_DIALECT_CRATE: &str = "syntaqlite-parser-sqlite";
const SQLITE_SHARED_CRATE: &str = "syntaqlite-parser";
const SQLITE_WRAPPERS_OUT: &str = "syntaqlite/src/parser/sqlite_wrappers.rs";

/// Output type for the dialect codegen command.
#[derive(clap::ValueEnum, Clone)]
pub(crate) enum OutputType {
    /// Dialect-only amalgamation (default).
    Dialect,
    /// Raw C/H/Rust files, flat layout.
    Raw,
    /// Runtime + dialect inlined into one self-contained file pair.
    Full,
    /// Runtime amalgamation only.
    RuntimeOnly,
    /// Internal workspace layout (hardcoded paths).
    Sqlite,
}

/// Generate dialect C sources and Rust bindings.
///
/// Base SQLite grammar and node files are embedded in the binary.
/// When `--actions-dir` / `--nodes-dir` are provided, those extension
/// files are merged with the base (same-name files replace the base).
#[derive(Parser)]
pub(crate) struct CodegenDialectArgs {
    /// Dialect identifier (e.g. "sqlite").
    #[arg(long, required = true)]
    name: String,

    /// Output directory for generated files (not used with --output-type=sqlite).
    #[arg(long)]
    output_dir: Option<String>,

    /// Directory containing .y grammar action files.
    #[arg(long)]
    actions_dir: Option<String>,

    /// Directory containing .synq node definitions.
    #[arg(long)]
    nodes_dir: Option<String>,

    /// Output type.
    #[arg(long, value_enum, default_value_t = OutputType::Dialect)]
    output_type: OutputType,

    /// Default path for the runtime header (dialect-only mode only).
    #[arg(long, default_value = "syntaqlite_runtime.h")]
    runtime_header: String,

    /// Default path for the extension header (dialect-only mode only).
    #[arg(long, default_value = "syntaqlite_dialect.h")]
    ext_header: String,
}

/// Hidden subcommands forwarded to the lemon/mkkeyword subprocess invocations.
///
/// These must be present in any binary that calls the codegen pipeline;
/// `generate_codegen_artifacts()` spawns the current executable with these.
#[derive(clap::Subcommand)]
pub(crate) enum ToolCommand {
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

pub(crate) fn dispatch_dialect(args: CodegenDialectArgs) -> Result<(), String> {
    match args.output_type {
        OutputType::Dialect => {
            let output_dir = args
                .output_dir
                .ok_or("--output-dir is required for --output-type=dialect")?;
            cmd_generate_dialect(
                &args.name,
                args.actions_dir.as_deref(),
                args.nodes_dir.as_deref(),
                &output_dir,
                &args.runtime_header,
                &args.ext_header,
            )
        }
        OutputType::Raw => {
            let output_dir = args
                .output_dir
                .ok_or("--output-dir is required for --output-type=raw")?;
            cmd_generate_dialect_raw(
                &args.name,
                args.actions_dir.as_deref(),
                args.nodes_dir.as_deref(),
                &output_dir,
            )
        }
        OutputType::Full => {
            let output_dir = args
                .output_dir
                .ok_or("--output-dir is required for --output-type=full")?;
            cmd_generate_dialect_full(
                &args.name,
                args.actions_dir.as_deref(),
                args.nodes_dir.as_deref(),
                &output_dir,
            )
        }
        OutputType::RuntimeOnly => {
            let output_dir = args
                .output_dir
                .ok_or("--output-dir is required for --output-type=runtime-only")?;
            cmd_generate_runtime(&output_dir)
        }
        OutputType::Sqlite => {
            if args.output_dir.is_some() {
                return Err(
                    "--output-dir must not be provided with --output-type=sqlite (uses hardcoded workspace paths)".to_string()
                );
            }
            cmd_generate_sqlite(args.actions_dir.as_deref(), args.nodes_dir.as_deref())
        }
    }
}

pub(crate) fn dispatch_tool(cmd: ToolCommand) -> Result<(), String> {
    match cmd {
        ToolCommand::Lemon { args } => syntaqlite_buildtools::run_lemon(&args),
        ToolCommand::Mkkeyword { args } => syntaqlite_buildtools::run_mkkeyword(&args),
    }
}

fn cmd_generate_dialect(
    dialect: &str,
    actions_dir: Option<&str>,
    nodes_dir: Option<&str>,
    output_dir: &str,
    runtime_header: &str,
    ext_header: &str,
) -> Result<(), String> {
    use syntaqlite_buildtools::amalgamate;

    let temp_dir = tempfile::TempDir::new().map_err(|e| format!("creating temp directory: {e}"))?;
    let temp = temp_dir.path();

    let (merged_y, merged_synq) = load_extensions(actions_dir, nodes_dir)?;

    let include_dir_name = format!("syntaqlite_{dialect}");
    codegen_to_dir_with_base(&merged_y, &merged_synq, temp, dialect, &include_dir_name)?;

    let out = Path::new(output_dir);
    ensure_dir(out, "output dir")?;
    let result =
        amalgamate::amalgamate_dialect(dialect, temp, Some(runtime_header), Some(ext_header))?;
    write_file(&out.join(format!("syntaqlite_{dialect}.h")), &result.header)?;
    write_file(&out.join(format!("syntaqlite_{dialect}.c")), &result.source)?;
    eprintln!("wrote {}/syntaqlite_{dialect}.{{h,c}}", out.display());
    Ok(())
}

fn cmd_generate_dialect_full(
    dialect: &str,
    actions_dir: Option<&str>,
    nodes_dir: Option<&str>,
    output_dir: &str,
) -> Result<(), String> {
    use syntaqlite_buildtools::amalgamate;

    let temp_dir = tempfile::TempDir::new().map_err(|e| format!("creating temp directory: {e}"))?;
    let temp = temp_dir.path();

    let (merged_y, merged_synq) = load_extensions(actions_dir, nodes_dir)?;

    let include_dir_name = format!("syntaqlite_{dialect}");
    codegen_to_dir_with_base(&merged_y, &merged_synq, temp, dialect, &include_dir_name)?;
    syntaqlite_buildtools::base_files::write_runtime_headers_to_dir(temp)
        .map_err(|e| format!("writing runtime headers: {e}"))?;

    let out = std::path::Path::new(output_dir);
    ensure_dir(out, "output dir")?;
    let result = amalgamate::amalgamate_full(dialect, temp, temp)?;
    write_file(&out.join(format!("syntaqlite_{dialect}.h")), &result.header)?;
    write_file(&out.join(format!("syntaqlite_{dialect}.c")), &result.source)?;
    eprintln!(
        "wrote {}/syntaqlite_{dialect}.{{h,c}} (full)",
        out.display()
    );
    Ok(())
}

fn cmd_generate_runtime(output_dir: &str) -> Result<(), String> {
    use syntaqlite_buildtools::amalgamate;

    let temp_dir = tempfile::TempDir::new().map_err(|e| format!("creating temp directory: {e}"))?;
    let temp = temp_dir.path();

    let (merged_y, merged_synq) = load_extensions(None, None)?;
    codegen_to_dir_with_base(&merged_y, &merged_synq, temp, "sqlite", "syntaqlite_sqlite")?;
    syntaqlite_buildtools::base_files::write_runtime_headers_to_dir(temp)
        .map_err(|e| format!("writing runtime headers: {e}"))?;

    let out = std::path::Path::new(output_dir);
    ensure_dir(out, "output dir")?;
    let result = amalgamate::amalgamate_runtime(temp)?;
    write_file(&out.join("syntaqlite_runtime.h"), &result.header)?;
    write_file(&out.join("syntaqlite_runtime.c"), &result.source)?;
    if let Some(ext) = &result.ext_header {
        write_file(&out.join("syntaqlite_dialect.h"), ext)?;
    }
    eprintln!(
        "wrote {}/syntaqlite_runtime.{{h,c}} + syntaqlite_dialect.h",
        out.display()
    );
    Ok(())
}

fn cmd_generate_dialect_raw(
    dialect: &str,
    actions_dir: Option<&str>,
    nodes_dir: Option<&str>,
    output_dir: &str,
) -> Result<(), String> {
    use syntaqlite_buildtools::codegen_api::{
        CodegenRequest, DialectNaming, extract_terminals_from_y, generate_codegen_artifacts,
    };
    use syntaqlite_buildtools::output_resolver::OutputLayout;

    let out = Path::new(output_dir);
    let (merged_y, merged_synq) = load_extensions(actions_dir, nodes_dir)?;
    let dialect_spec = DialectNaming::new(dialect);
    let parser_prefix = dialect_spec.parser_symbol_prefix();
    let ext_y_contents: Vec<&str> = merged_y
        .iter()
        .filter(|(name, _)| {
            !syntaqlite_buildtools::base_files::base_y_files()
                .iter()
                .any(|(base_name, _)| *base_name == name.as_str())
        })
        .map(|(_, content)| content.as_str())
        .collect();
    let extra_keywords = extract_terminals_from_y(&ext_y_contents);

    let layout = OutputLayout::for_external(out, dialect, &dialect_spec.include_dir_name());

    let artifacts = {
        let request = CodegenRequest {
            dialect: &dialect_spec,
            y_files: &merged_y,
            synq_files: &merged_synq,
            extra_keywords: &extra_keywords,
            parser_symbol_prefix: Some(&parser_prefix),
            include_rust: false,
            crate_name: None,
            base_synq_files: Some(syntaqlite_buildtools::base_files::base_synq_files()),
            open_for_extension: false,
            dialect_c_includes: layout.c_includes(),
            internal_wrappers: false,
        };
        generate_codegen_artifacts(&request)?
    };

    layout.write_codegen_artifacts(
        &dialect_spec,
        artifacts,
        &|dir| ensure_dir(dir, "output directory"),
        &|path, content| write_file(path, content),
    )?;

    eprintln!("wrote raw dialect files to {}", out.display());
    Ok(())
}

fn cmd_generate_sqlite(
    actions_dir: Option<&str>,
    nodes_dir: Option<&str>,
) -> Result<(), String> {
    use syntaqlite_buildtools::codegen_api::{
        CodegenRequest, DialectNaming, generate_codegen_artifacts, read_named_files_from_dir,
    };
    use syntaqlite_buildtools::output_resolver::OutputLayout;

    let dialect = DialectNaming::new("sqlite");
    let y_files = match actions_dir {
        Some(dir) => read_named_files_from_dir(dir, "y")?,
        None => return Err("--actions-dir is required for --output-type=sqlite".to_string()),
    };
    let synq_files = match nodes_dir {
        Some(dir) => read_named_files_from_dir(dir, "synq")?,
        None => return Err("--nodes-dir is required for --output-type=sqlite".to_string()),
    };

    let mut layout = OutputLayout::for_sqlite(
        Path::new("."),
        SQLITE_DIALECT_CRATE,
        SQLITE_SHARED_CRATE,
        dialect.name(),
        &dialect.include_dir_name(),
        Some(SQLITE_WRAPPERS_OUT),
    );
    // ast_traits_rs is written separately by codegen-sqlite-parser
    layout.ast_traits_rs = None;

    let artifacts = {
        let no_keywords: Vec<String> = Vec::new();
        let request = CodegenRequest {
            dialect: &dialect,
            y_files: &y_files,
            synq_files: &synq_files,
            extra_keywords: &no_keywords,
            parser_symbol_prefix: None,
            include_rust: true,
            crate_name: Some("syntaqlite_parser"),
            base_synq_files: None,
            open_for_extension: true,
            dialect_c_includes: layout.c_includes(),
            internal_wrappers: true,
        };
        generate_codegen_artifacts(&request)?
    };

    // Clean stale generated C/H files from C output directories.
    let csrc_dir = Path::new(SQLITE_DIALECT_CRATE).join("csrc/sqlite");
    let include_dir = Path::new(SQLITE_DIALECT_CRATE)
        .join(format!("include/{}", dialect.include_dir_name()));
    let shared_include_dir =
        Path::new(SQLITE_SHARED_CRATE).join(format!("include/{}", dialect.include_dir_name()));
    for dir in [&csrc_dir, &include_dir, &shared_include_dir] {
        if dir.is_dir() {
            clean_generated_files(dir);
        }
    }

    layout.write_codegen_artifacts(
        &dialect,
        artifacts,
        &|dir| ensure_dir(dir, "output directory"),
        &|path, content| write_file(path, content),
    )?;

    Ok(())
}

/// Delete any .c/.h files in `dir` whose first 512 bytes contain the autogenerated marker.
fn clean_generated_files(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "c" && ext != "h" {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let prefix = &content[..content.len().min(512)];
        if prefix.contains(syntaqlite_buildtools::codegen_api::AUTOGENERATED_MARKER) {
            let _ = fs::remove_file(&path);
        }
    }
}

/// A set of named files: `(filename, content)` pairs.
type NamedFiles = Vec<(String, String)>;

/// Load extension `.y` and `.synq` files and merge them with the base file sets.
fn load_extensions(
    actions_dir: Option<&str>,
    nodes_dir: Option<&str>,
) -> Result<(NamedFiles, NamedFiles), String> {
    use syntaqlite_buildtools::base_files;

    let ext_y = match actions_dir {
        Some(dir) => syntaqlite_buildtools::codegen_api::read_named_files_from_dir(dir, "y")?,
        None => Vec::new(),
    };
    let ext_synq = match nodes_dir {
        Some(dir) => syntaqlite_buildtools::codegen_api::read_named_files_from_dir(dir, "synq")?,
        None => Vec::new(),
    };

    let merged_y = base_files::merge_file_sets(base_files::base_y_files(), &ext_y);
    let merged_synq = base_files::merge_file_sets(base_files::base_synq_files(), &ext_synq);
    Ok((merged_y, merged_synq))
}

/// Run the codegen pipeline from merged in-memory file sets into a temp directory.
fn codegen_to_dir_with_base(
    y_files: &[(String, String)],
    synq_files: &[(String, String)],
    temp_root: &Path,
    dialect_name: &str,
    include_dir_name: &str,
) -> Result<(), String> {
    use syntaqlite_buildtools::codegen_api::{
        CodegenRequest, DialectNaming, extract_terminals_from_y, generate_codegen_artifacts,
    };
    use syntaqlite_buildtools::output_resolver::OutputLayout;

    let dialect_spec = DialectNaming::new(dialect_name);
    let parser_prefix = dialect_spec.parser_symbol_prefix();
    let ext_y_contents: Vec<&str> = y_files
        .iter()
        .filter(|(name, _)| {
            !syntaqlite_buildtools::base_files::base_y_files()
                .iter()
                .any(|(base_name, _)| *base_name == name.as_str())
        })
        .map(|(_, content)| content.as_str())
        .collect();
    let extra_keywords = extract_terminals_from_y(&ext_y_contents);

    let layout = OutputLayout::for_amalg_temp(temp_root, dialect_name, include_dir_name);

    let artifacts = {
        let request = CodegenRequest {
            dialect: &dialect_spec,
            y_files,
            synq_files,
            extra_keywords: &extra_keywords,
            parser_symbol_prefix: Some(&parser_prefix),
            include_rust: false,
            crate_name: None,
            base_synq_files: Some(syntaqlite_buildtools::base_files::base_synq_files()),
            open_for_extension: false,
            dialect_c_includes: layout.c_includes(),
            internal_wrappers: false,
        };
        generate_codegen_artifacts(&request)?
    };

    layout.write_codegen_artifacts(
        &dialect_spec,
        artifacts,
        &|dir| ensure_dir(dir, "output directory"),
        &|path, content| write_file(path, content),
    )?;
    Ok(())
}

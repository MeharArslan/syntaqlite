// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Codegen subcommands: dialect codegen and internal SQLite-parser artifacts.

use std::fs;
use std::io::Read;
use std::path::Path;

fn ensure_dir(path: &Path, label: &str) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("Failed to create {label}: {e}"))
}

fn write_file(path: &Path, content: impl AsRef<[u8]>) -> Result<(), String> {
    fs::write(path, content).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

// ── dialect codegen ──────────────────────────────────────────────────────────

// Hardcoded workspace paths for --output-type=sqlite
const SQLITE_DIALECT_CRATE: &str = "syntaqlite-parser-sqlite";
const SQLITE_SHARED_CRATE: &str = "syntaqlite-parser";
const SQLITE_WRAPPERS_OUT: &str = "syntaqlite-parser-sqlite/src/wrappers.rs";

/// Output type for the dialect codegen command.
#[cfg(feature = "codegen-dialect")]
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
#[cfg(feature = "codegen-dialect")]
#[derive(clap::Parser)]
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
#[cfg(feature = "codegen-dialect")]
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

#[cfg(feature = "codegen-dialect")]
pub(crate) fn dispatch_dialect(args: CodegenDialectArgs) -> Result<(), String> {
    let name = &args.name;
    let actions_dir = args.actions_dir.as_deref();
    let nodes_dir = args.nodes_dir.as_deref();
    let require_output_dir = |type_name: &str| -> Result<String, String> {
        args.output_dir
            .clone()
            .ok_or_else(|| format!("--output-dir is required for --output-type={type_name}"))
    };

    match args.output_type {
        OutputType::Dialect => cmd_generate_dialect(
            name,
            actions_dir,
            nodes_dir,
            &require_output_dir("dialect")?,
            &args.runtime_header,
            &args.ext_header,
        ),
        OutputType::Raw => {
            cmd_generate_dialect_raw(name, actions_dir, nodes_dir, &require_output_dir("raw")?)
        }
        OutputType::Full => {
            cmd_generate_dialect_full(name, actions_dir, nodes_dir, &require_output_dir("full")?)
        }
        OutputType::RuntimeOnly => cmd_generate_runtime(&require_output_dir("runtime-only")?),
        OutputType::Sqlite => {
            if args.output_dir.is_some() {
                return Err(
                    "--output-dir must not be provided with --output-type=sqlite (uses hardcoded workspace paths)".to_string()
                );
            }
            cmd_generate_sqlite(actions_dir, nodes_dir)
        }
    }
}

#[cfg(feature = "codegen-dialect")]
pub(crate) fn dispatch_tool(cmd: ToolCommand) -> Result<(), String> {
    match cmd {
        ToolCommand::Lemon { args } => syntaqlite_buildtools::run_lemon(&args),
        ToolCommand::Mkkeyword { args } => syntaqlite_buildtools::run_mkkeyword(&args),
    }
}

#[cfg(feature = "codegen-dialect")]
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
    codegen_to_dir_with_base(&merged_y, &merged_synq, temp, dialect)?;

    let out = Path::new(output_dir);
    ensure_dir(out, "output dir")?;
    let result =
        amalgamate::amalgamate_dialect(dialect, temp, Some(runtime_header), Some(ext_header))?;
    write_file(&out.join(format!("syntaqlite_{dialect}.h")), &result.header)?;
    write_file(&out.join(format!("syntaqlite_{dialect}.c")), &result.source)?;
    eprintln!("wrote {}/syntaqlite_{dialect}.{{h,c}}", out.display());
    Ok(())
}

#[cfg(feature = "codegen-dialect")]
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
    codegen_to_dir_with_base(&merged_y, &merged_synq, temp, dialect)?;
    syntaqlite_buildtools::base_files::write_runtime_headers_to_dir(temp)
        .map_err(|e| format!("writing runtime headers: {e}"))?;

    let out = Path::new(output_dir);
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

#[cfg(feature = "codegen-dialect")]
fn cmd_generate_runtime(output_dir: &str) -> Result<(), String> {
    use syntaqlite_buildtools::amalgamate;

    let temp_dir = tempfile::TempDir::new().map_err(|e| format!("creating temp directory: {e}"))?;
    let temp = temp_dir.path();
    let (merged_y, merged_synq) = load_extensions(None, None)?;
    codegen_to_dir_with_base(&merged_y, &merged_synq, temp, "sqlite")?;
    syntaqlite_buildtools::base_files::write_runtime_headers_to_dir(temp)
        .map_err(|e| format!("writing runtime headers: {e}"))?;

    let out = Path::new(output_dir);
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

#[cfg(feature = "codegen-dialect")]
fn cmd_generate_dialect_raw(
    dialect: &str,
    actions_dir: Option<&str>,
    nodes_dir: Option<&str>,
    output_dir: &str,
) -> Result<(), String> {
    use syntaqlite_buildtools::codegen_api::{DialectCodegenJob, DialectNaming};
    use syntaqlite_buildtools::output_resolver::OutputLayout;

    let (merged_y, merged_synq) = load_extensions(actions_dir, nodes_dir)?;
    let dialect_spec = DialectNaming::new(dialect);
    let layout = OutputLayout::for_external(
        Path::new(output_dir),
        dialect,
        &dialect_spec.include_dir_name(),
    );
    DialectCodegenJob::new(&dialect_spec, &merged_y, &merged_synq)
        .with_base_synq(syntaqlite_buildtools::base_files::base_synq_files())
        .write_to(
            &layout,
            &|dir| ensure_dir(dir, "output directory"),
            &|path, content| write_file(path, content),
        )?;
    eprintln!("wrote raw dialect files to {output_dir}");
    Ok(())
}

#[cfg(feature = "codegen-dialect")]
fn cmd_generate_sqlite(actions_dir: Option<&str>, nodes_dir: Option<&str>) -> Result<(), String> {
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
    let include_dir =
        Path::new(SQLITE_DIALECT_CRATE).join(format!("include/{}", dialect.include_dir_name()));
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
#[cfg(feature = "codegen-dialect")]
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
        let Ok(mut file) = fs::File::open(&path) else {
            continue;
        };
        let mut buf = [0u8; 512];
        let n = file.read(&mut buf).unwrap_or(0);
        let prefix = std::str::from_utf8(&buf[..n]).unwrap_or("");
        if prefix.contains(syntaqlite_buildtools::codegen_api::AUTOGENERATED_MARKER) {
            let _ = fs::remove_file(&path);
        }
    }
}

/// A set of named files: `(filename, content)` pairs.
#[cfg(feature = "codegen-dialect")]
type NamedFiles = Vec<(String, String)>;

/// Load extension `.y` and `.synq` files and merge them with the base file sets.
#[cfg(feature = "codegen-dialect")]
fn load_extensions(
    actions_dir: Option<&str>,
    nodes_dir: Option<&str>,
) -> Result<(NamedFiles, NamedFiles), String> {
    use syntaqlite_buildtools::base_files;
    use syntaqlite_buildtools::codegen_api::read_named_files_from_dir;

    let read_ext = |dir: Option<&str>, ext: &str| -> Result<NamedFiles, String> {
        Ok(match dir {
            Some(d) => read_named_files_from_dir(d, ext)?,
            None => Vec::new(),
        })
    };
    let merged_y =
        base_files::merge_file_sets(base_files::base_y_files(), &read_ext(actions_dir, "y")?);
    let merged_synq =
        base_files::merge_file_sets(base_files::base_synq_files(), &read_ext(nodes_dir, "synq")?);
    Ok((merged_y, merged_synq))
}

/// Run the codegen pipeline from merged in-memory file sets into a temp directory.
#[cfg(feature = "codegen-dialect")]
fn codegen_to_dir_with_base(
    y_files: &NamedFiles,
    synq_files: &NamedFiles,
    temp_root: &Path,
    dialect_name: &str,
) -> Result<(), String> {
    use syntaqlite_buildtools::codegen_api::{DialectCodegenJob, DialectNaming};
    use syntaqlite_buildtools::output_resolver::OutputLayout;

    let dialect_spec = DialectNaming::new(dialect_name);
    let layout =
        OutputLayout::for_amalg_temp(temp_root, dialect_name, &dialect_spec.include_dir_name());
    DialectCodegenJob::new(&dialect_spec, y_files, synq_files)
        .with_base_synq(syntaqlite_buildtools::base_files::base_synq_files())
        .write_to(
            &layout,
            &|dir| ensure_dir(dir, "output directory"),
            &|path, content| write_file(path, content),
        )
}

// ── internal SQLite-parser artifacts ─────────────────────────────────────────

/// Generate internal Rust artifacts for the SQLite parser crate.
///
/// This is a flat (non-subcommand) command. It generates three internal-only
/// Rust artifacts from pre-existing inputs:
///   - functions catalog (from functions.json)
///   - ast_traits.rs (from synq + actions files via the full codegen pipeline)
///   - cflag versions table (from a pre-computed cflag audit JSON)
#[cfg(feature = "internal")]
#[derive(clap::Parser)]
pub(crate) struct SqliteParserArgs {
    /// Path to functions.json (from sqlite-vendored/data/functions.json).
    #[arg(long)]
    functions_json: Option<String>,

    /// Path to the cflag audit JSON (optional; required if --cflag-versions-out is given).
    #[arg(long)]
    cflag_audit_json: Option<String>,

    /// Directory containing .y grammar action files (needed for --ast-traits-out).
    #[arg(long)]
    actions_dir: Option<String>,

    /// Directory containing .synq node definitions (needed for --ast-traits-out).
    #[arg(long)]
    nodes_dir: Option<String>,

    /// Output path for the generated ast_traits.rs.
    #[arg(long)]
    ast_traits_out: Option<String>,

    /// Output path for the generated functions_catalog.rs.
    #[arg(long)]
    functions_catalog_out: Option<String>,

    /// Output path for the generated cflag versions table Rust file.
    /// Requires --cflag-audit-json.
    #[arg(long, requires = "cflag_audit_json")]
    cflag_versions_out: Option<String>,
}

#[cfg(feature = "internal")]
pub(crate) fn dispatch_sqlite_parser(args: SqliteParserArgs) -> Result<(), String> {
    if let Some(catalog_out) = &args.functions_catalog_out {
        let json_path = args
            .functions_json
            .as_deref()
            .ok_or("--functions-json is required when --functions-catalog-out is given")?;
        syntaqlite_buildtools::util::functions_codegen::write_functions_catalog_file(
            json_path,
            catalog_out,
        )?;
    }

    if let Some(traits_out) = &args.ast_traits_out {
        let actions_dir = args
            .actions_dir
            .as_deref()
            .ok_or("--actions-dir is required when --ast-traits-out is given")?;
        let nodes_dir = args
            .nodes_dir
            .as_deref()
            .ok_or("--nodes-dir is required when --ast-traits-out is given")?;
        generate_ast_traits(actions_dir, nodes_dir, traits_out)?;
    }

    if let Some(cflag_out) = &args.cflag_versions_out {
        let audit_path = args
            .cflag_audit_json
            .as_deref()
            .ok_or("--cflag-audit-json is required when --cflag-versions-out is given")?;
        generate_cflag_versions(audit_path, cflag_out)?;
    }

    Ok(())
}

#[cfg(feature = "internal")]
fn generate_ast_traits(
    actions_dir: &str,
    nodes_dir: &str,
    output_path: &str,
) -> Result<(), String> {
    use syntaqlite_buildtools::codegen_api::{
        CodegenRequest, DialectNaming, generate_codegen_artifacts, read_named_files_from_dir,
    };
    use syntaqlite_buildtools::output_resolver::OutputLayout;

    let dialect = DialectNaming::new("sqlite");
    let y_files = read_named_files_from_dir(actions_dir, "y")?;
    let synq_files = read_named_files_from_dir(nodes_dir, "synq")?;

    // We need a layout just to get the c_includes; use the same sqlite layout but
    // we only care about extracting the ast_traits field.
    let layout = OutputLayout::for_sqlite(
        Path::new("."),
        SQLITE_DIALECT_CRATE,
        SQLITE_SHARED_CRATE,
        dialect.name(),
        &dialect.include_dir_name(),
        None,
    );

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

    let artifacts = generate_codegen_artifacts(&request)?;

    let ast_traits_content = artifacts
        .rust
        .as_ref()
        .and_then(|r| r.ast_traits_rs.as_deref())
        .ok_or("codegen did not produce ast_traits_rs")?;

    let out = Path::new(output_path);
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("creating output directory: {e}"))?;
    }
    fs::write(out, ast_traits_content).map_err(|e| format!("writing {}: {e}", out.display()))?;
    eprintln!("wrote {output_path}");
    Ok(())
}

#[cfg(feature = "internal")]
fn generate_cflag_versions(audit_json_path: &str, output_path: &str) -> Result<(), String> {
    use syntaqlite_buildtools::extract::functions::{CflagAvailability, write_cflag_versions_rs};

    let audit_json = fs::read_to_string(audit_json_path)
        .map_err(|e| format!("reading {audit_json_path}: {e}"))?;
    let availability: CflagAvailability =
        serde_json::from_str(&audit_json).map_err(|e| format!("parsing cflag audit JSON: {e}"))?;
    write_cflag_versions_rs(&availability, Path::new(output_path))?;
    eprintln!("wrote {output_path}");
    Ok(())
}

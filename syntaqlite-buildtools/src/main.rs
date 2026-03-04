// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::similar_names))]

//! Internal bootstrap and code generation tool.
//!
//! This binary has no dependency on any generated files, so it can be built
//! from a completely clean checkout and used to regenerate everything.
//!
//! Subcommands:
//!   codegen-sqlite          — regenerate the internal `SQLite` dialect (stage 2)
//!   codegen-sqlite-parser   — regenerate `ast_traits`, `functions_catalog` (stage 1b)
//!   sqlite-extract          — extract C fragments from raw `SQLite` source (stage 1)
//!   audit-cflags            — audit cflag versions across `SQLite` amalgamations
//!   generate-functions-catalog — generate Rust functions catalog from functions.json
//!   extract-functions       — extract function catalog from `SQLite` amalgamations
//!   analyze-versions        — analyze `SQLite` source version history

use std::fs;
use std::path::Path;

use clap::{Parser, Subcommand};

fn write_file(path: &Path, content: impl AsRef<[u8]>) -> Result<(), String> {
    fs::write(path, content).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

fn ensure_dir(path: &Path, label: &str) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("Failed to create {label}: {e}"))
}

// Hardcoded workspace paths for the internal SQLite dialect.
const SQLITE_DIALECT_CRATE: &str = "syntaqlite-syntax";
const SQLITE_SHARED_CRATE: &str = "syntaqlite-syntax";
const SQLITE_FUNCTIONS_CATALOG: &str = "syntaqlite/src/sqlite/functions_catalog.rs";

#[derive(Parser)]
#[command(about = "Internal bootstrap and code generation tool for syntaqlite")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Regenerate the internal `SQLite` dialect C sources and Rust bindings (stage 2).
    ///
    /// Writes generated files to the hardcoded workspace paths in
    /// syntaqlite-syntax/.
    #[command(name = "codegen-sqlite")]
    CodegenSqlite(CodegenSqliteArgs),

    /// Regenerate internal Rust artifacts for the `SQLite` parser crate (stage 1b).
    ///
    /// Generates `ast_traits.rs`, `functions_catalog.rs`, and optionally
    /// `cflag_versions.rs` from pre-existing inputs.
    #[command(name = "codegen-sqlite-parser")]
    CodegenSqliteParser(CodegenSqliteParserArgs),

    /// Extract C fragments from raw `SQLite` source (stage 1).
    #[command(name = "sqlite-extract")]
    SqliteExtract(SqliteExtractArgs),

    /// Audit which compile flags each `SQLite` amalgamation version references.
    #[command(name = "audit-cflags")]
    AuditCflags(AuditCflagsArgs),

    /// Generate the Rust functions catalog module from functions.json.
    #[command(name = "generate-functions-catalog")]
    GenerateFunctionsCatalog(GenerateFunctionsCatalogArgs),

    /// Extract built-in function catalog from pre-downloaded `SQLite` amalgamations.
    #[command(name = "extract-functions")]
    ExtractFunctions(ExtractFunctionsArgs),

    /// Analyze multiple `SQLite` source versions to find fragment variants.
    #[command(name = "analyze-versions")]
    AnalyzeVersions(AnalyzeVersionsArgs),

    /// Hidden subprocess entry point for the Lemon parser generator.
    #[command(hide = true)]
    Lemon {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Hidden subprocess entry point for the mkkeyword hash generator.
    #[command(hide = true)]
    Mkkeyword {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

// ── codegen-sqlite ────────────────────────────────────────────────────────────

#[derive(clap::Args)]
struct CodegenSqliteArgs {
    /// Directory containing .y grammar action files.
    #[arg(long, required = true)]
    actions_dir: String,

    /// Directory containing .synq node definitions.
    #[arg(long, required = true)]
    nodes_dir: String,
}

fn cmd_codegen_sqlite(args: &CodegenSqliteArgs) -> Result<(), String> {
    use syntaqlite_buildtools::codegen_api::{
        CodegenRequest, DialectNaming, generate_codegen_artifacts, read_named_files_from_dir,
    };
    use syntaqlite_buildtools::output_resolver::OutputLayout;

    let dialect = DialectNaming::new("sqlite");
    let y_files = read_named_files_from_dir(&args.actions_dir, "y")?;
    let synq_files = read_named_files_from_dir(&args.nodes_dir, "synq")?;

    let mut layout = OutputLayout::for_sqlite(
        Path::new("."),
        SQLITE_DIALECT_CRATE,
        SQLITE_SHARED_CRATE,
        dialect.name(),
        &dialect.include_dir_name(),
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
            crate_name: Some("crate"),
            base_synq_files: None,
            open_for_extension: true,
            dialect_c_includes: layout.c_includes(),
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
fn clean_generated_files(dir: &Path) {
    use std::io::Read;

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

// ── codegen-sqlite-parser ─────────────────────────────────────────────────────

#[derive(clap::Args)]
struct CodegenSqliteParserArgs {
    /// Path to functions.json (from sqlite-vendored/data/functions.json).
    /// When provided, generates `functions_catalog.rs` at its hardcoded workspace path.
    #[arg(long)]
    functions_json: Option<String>,

    /// Path to the cflag audit JSON (optional; required if --cflag-versions-out is given).
    #[arg(long)]
    cflag_audit_json: Option<String>,

    /// Directory containing .y grammar action files.
    /// When provided together with --nodes-dir, generates `ast_traits.rs`.
    #[arg(long)]
    actions_dir: Option<String>,

    /// Directory containing .synq node definitions.
    /// When provided together with --actions-dir, generates `ast_traits.rs`.
    #[arg(long)]
    nodes_dir: Option<String>,

    /// Output path for the generated cflag versions table Rust file.
    /// Requires --cflag-audit-json.
    #[arg(long, requires = "cflag_audit_json")]
    cflag_versions_out: Option<String>,
}

fn cmd_codegen_sqlite_parser(args: &CodegenSqliteParserArgs) -> Result<(), String> {
    if let Some(json_path) = &args.functions_json {
        syntaqlite_buildtools::util::functions_codegen::write_functions_catalog_file(
            json_path,
            SQLITE_FUNCTIONS_CATALOG,
        )?;
    }

    if let (Some(actions_dir), Some(nodes_dir)) =
        (args.actions_dir.as_deref(), args.nodes_dir.as_deref())
    {
        cmd_generate_ast_traits(actions_dir, nodes_dir)?;
    }

    if let Some(cflag_out) = &args.cflag_versions_out {
        let audit_path = args
            .cflag_audit_json
            .as_deref()
            .ok_or("--cflag-audit-json is required when --cflag-versions-out is given")?;
        cmd_generate_cflag_versions(audit_path, cflag_out)?;
    }

    Ok(())
}

fn cmd_generate_ast_traits(actions_dir: &str, nodes_dir: &str) -> Result<(), String> {
    use syntaqlite_buildtools::codegen_api::{
        CodegenRequest, DialectNaming, generate_codegen_artifacts, read_named_files_from_dir,
    };
    use syntaqlite_buildtools::output_resolver::OutputLayout;

    let dialect = DialectNaming::new("sqlite");
    let y_files = read_named_files_from_dir(actions_dir, "y")?;
    let synq_files = read_named_files_from_dir(nodes_dir, "synq")?;

    let layout = OutputLayout::for_sqlite(
        Path::new("."),
        SQLITE_DIALECT_CRATE,
        SQLITE_SHARED_CRATE,
        dialect.name(),
        &dialect.include_dir_name(),
    );

    let no_keywords: Vec<String> = Vec::new();
    let request = CodegenRequest {
        dialect: &dialect,
        y_files: &y_files,
        synq_files: &synq_files,
        extra_keywords: &no_keywords,
        parser_symbol_prefix: None,
        include_rust: true,
        crate_name: Some("syntaqlite_syntax"),
        base_synq_files: None,
        open_for_extension: true,
        dialect_c_includes: layout.c_includes(),
    };

    let artifacts = generate_codegen_artifacts(&request)?;

    let ast_traits_content = artifacts
        .rust
        .as_ref()
        .and_then(|r| r.ast_traits_rs.as_deref())
        .ok_or("codegen did not produce ast_traits_rs")?;

    let output_path = layout
        .ast_traits_rs
        .as_deref()
        .ok_or("layout has no ast_traits_rs path")?;
    let out = Path::new(output_path);
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("creating output directory: {e}"))?;
    }
    fs::write(out, ast_traits_content).map_err(|e| format!("writing {}: {e}", out.display()))?;
    eprintln!("wrote {output_path}");
    Ok(())
}

fn cmd_generate_cflag_versions(audit_json_path: &str, output_path: &str) -> Result<(), String> {
    use syntaqlite_buildtools::extract::functions::{CflagAvailability, write_cflag_versions_rs};

    let audit_json = fs::read_to_string(audit_json_path)
        .map_err(|e| format!("reading {audit_json_path}: {e}"))?;
    let availability: CflagAvailability =
        serde_json::from_str(&audit_json).map_err(|e| format!("parsing cflag audit JSON: {e}"))?;
    write_cflag_versions_rs(&availability, Path::new(output_path))?;
    eprintln!("wrote {output_path}");
    Ok(())
}

// ── sqlite-extract ────────────────────────────────────────────────────────────

#[derive(clap::Args)]
struct SqliteExtractArgs {
    /// Path to the `SQLite` source tree root (containing src/ and tool/).
    #[arg(long, required = true)]
    sqlite_src: String,
    /// Output directory for vendored files (sqlite-vendored/).
    #[arg(long, required = true)]
    output_dir: String,
    /// Path to the parser-actions directory (containing .y files).
    #[arg(long, required = true)]
    actions_dir: String,
    /// Path to the parser-nodes directory (containing .synq files).
    #[arg(long, required = true)]
    nodes_dir: String,
}

fn cmd_sqlite_extract(args: &SqliteExtractArgs) -> Result<(), String> {
    use syntaqlite_buildtools::extract;

    let root = Path::new(&args.sqlite_src);
    if !root.is_dir() {
        return Err(format!("{} is not a directory", args.sqlite_src));
    }

    let src = root.join("src");
    let tool_dir = root.join("tool");
    let out = Path::new(&args.output_dir);
    let sources_dir = out.join("sources");
    let fragments_dir = sources_dir.join("fragments");
    let data_dir = out.join("data");

    for dir in [&sources_dir, &fragments_dir, &data_dir] {
        fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    }

    eprintln!("Vendoring SQLite tool sources...");
    for filename in &["lemon.c", "lempar.c", "mkkeywordhash.c"] {
        let src_path = tool_dir.join(filename);
        let dest_path = sources_dir.join(filename);
        fs::copy(&src_path, &dest_path).map_err(|e| {
            format!(
                "copying {} -> {}: {e}",
                src_path.display(),
                dest_path.display()
            )
        })?;
        eprintln!("  {filename}");
    }

    eprintln!("Transforming mkkeywordhash.c...");
    let mkkeywordhash_c = read_source(&tool_dir, "mkkeywordhash.c")?;
    extract::mkkeywordhash::write_modified_mkkeywordhash(
        &mkkeywordhash_c,
        &sources_dir.join("mkkeywordhash_modified.c"),
    )?;

    eprintln!("Extracting tokenizer fragments...");
    let tokenize_c = read_source(&src, "tokenize.c")?;
    let global_c = read_source(&src, "global.c")?;
    let sqliteint_h = read_source(&src, "sqliteInt.h")?;
    let fragments = extract::tokenizer::extract_fragments(&tokenize_c, &global_c, &sqliteint_h)?;
    extract::tokenizer::write_fragments(&fragments, &fragments_dir)?;

    eprintln!("Extracting keyword cflag data...");
    let cflags = extract::keywords_and_parser::extract_keyword_cflags(&mkkeywordhash_c)?;
    extract::keywords_and_parser::write_keyword_cflags(&cflags, &data_dir.join("cflags.json"))?;

    eprintln!("Generating base_files_tables.rs...");
    let codegen_crate_dir = Path::new(&args.output_dir)
        .parent()
        .ok_or("output_dir has no parent")?;
    let tables_path = codegen_crate_dir.join("src/base_files_tables.rs");
    extract::base_files::write_base_files_tables(
        Path::new(&args.actions_dir),
        Path::new(&args.nodes_dir),
        codegen_crate_dir,
        &tables_path,
    )?;

    eprintln!("Stage 1 complete. Output: {}", out.display());
    Ok(())
}

fn read_source(dir: &Path, name: &str) -> Result<String, String> {
    let path = dir.join(name);
    fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))
}

// ── audit-cflags ──────────────────────────────────────────────────────────────

#[derive(clap::Args)]
struct AuditCflagsArgs {
    /// Directory containing amalgamations (e.g., sqlite-amalgamations/3.35.5/sqlite3.c).
    #[arg(long, required = true)]
    amalgamation_dir: String,
    /// Output path for the audit JSON.
    #[arg(long, required = true)]
    output: String,
    /// Output path for the generated Rust cflag versions table.
    #[arg(long, required = true)]
    rust_output: String,
}

fn cmd_audit_cflags(args: &AuditCflagsArgs) -> Result<(), String> {
    use syntaqlite_buildtools::extract;

    let amal_path = Path::new(&args.amalgamation_dir);
    if !amal_path.is_dir() {
        return Err(format!("{} is not a directory", args.amalgamation_dir));
    }
    let availability =
        extract::functions::audit_version_cflags(amal_path, Path::new(&args.output))?;
    extract::functions::write_cflag_versions_rs(&availability, Path::new(&args.rust_output))
}

// ── generate-functions-catalog ────────────────────────────────────────────────

#[derive(clap::Args)]
struct GenerateFunctionsCatalogArgs {
    /// Path to functions.json (from extract-functions).
    #[arg(long, required = true)]
    functions_json: String,
    /// Output path for the generated Rust file.
    #[arg(long, required = true)]
    output: String,
}

fn cmd_generate_functions_catalog(args: &GenerateFunctionsCatalogArgs) -> Result<(), String> {
    syntaqlite_buildtools::util::functions_codegen::write_functions_catalog_file(
        &args.functions_json,
        &args.output,
    )
}

// ── extract-functions ─────────────────────────────────────────────────────────

#[derive(clap::Args)]
struct ExtractFunctionsArgs {
    /// Directory containing amalgamations (e.g., sqlite-amalgamations/3.35.5/sqlite3.c).
    #[arg(long, required = true)]
    amalgamation_dir: String,
    /// Path to the cflag audit JSON (from audit-cflags).
    #[arg(long, required = true)]
    audit: String,
    /// Output path for the function catalog JSON.
    #[arg(long, required = true)]
    output: String,
}

fn cmd_extract_functions(args: &ExtractFunctionsArgs) -> Result<(), String> {
    use syntaqlite_buildtools::extract;

    let amal_path = Path::new(&args.amalgamation_dir);
    if !amal_path.is_dir() {
        return Err(format!("{} is not a directory", args.amalgamation_dir));
    }
    extract::functions::extract_function_catalog(
        amal_path,
        Path::new(&args.audit),
        Path::new(&args.output),
    )
    .map(|_| ())
}

// ── analyze-versions ──────────────────────────────────────────────────────────

#[derive(clap::Args)]
struct AnalyzeVersionsArgs {
    /// Directory containing per-version `SQLite` source trees.
    /// Expected layout: <dir>/3.35.0/src/tokenize.c, etc.
    #[arg(long, required = true)]
    sqlite_source_dir: String,
    /// Output directory for variant files.
    #[arg(long, required = true)]
    output_dir: String,
}

fn cmd_analyze_versions(args: &AnalyzeVersionsArgs) -> Result<(), String> {
    let source_dir = Path::new(&args.sqlite_source_dir);
    let out_dir = Path::new(&args.output_dir);

    fs::create_dir_all(out_dir).map_err(|e| format!("failed to create output dir: {e}"))?;

    let analysis = syntaqlite_buildtools::version_analysis::analyze_versions(source_dir, out_dir)?;

    let json = serde_json::to_string_pretty(&analysis)
        .map_err(|e| format!("JSON serialization failed: {e}"))?;
    println!("{json}");

    eprintln!(
        "Analysis complete: {} versions, {} fragments analyzed",
        analysis.versions.len(),
        analysis.fragments.len()
    );
    for (name, frag) in &analysis.fragments {
        let errors = if frag.errors.is_empty() {
            String::new()
        } else {
            format!(", {} errors", frag.errors.len())
        };
        eprintln!("  {name}: {} variant(s){errors}", frag.variants.len());
    }
    eprintln!(
        "  keywords: {} total, {} addition points",
        analysis.keywords.total_keywords_latest,
        analysis.keywords.additions.len()
    );
    if let Some(ref grammar) = analysis.grammar {
        eprintln!(
            "  grammar: {} versions parsed, {} change points, {} errors",
            grammar.per_version.len(),
            grammar.diffs.len(),
            grammar.errors.len()
        );
    }
    eprintln!("Variant files written to {}", out_dir.display());

    if let Some(ref grammar) = analysis.grammar {
        let report =
            syntaqlite_buildtools::version_analysis::grammar::format_grammar_report(grammar);
        let report_path = out_dir.join("grammar_report.md");
        fs::write(&report_path, &report)
            .map_err(|e| format!("write {}: {e}", report_path.display()))?;
        eprintln!("Grammar report written to {}", report_path.display());
    }

    Ok(())
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    let result: Result<(), String> = match &cli.command {
        Command::CodegenSqlite(args) => cmd_codegen_sqlite(args),
        Command::CodegenSqliteParser(args) => cmd_codegen_sqlite_parser(args),
        Command::SqliteExtract(args) => cmd_sqlite_extract(args),
        Command::AuditCflags(args) => cmd_audit_cflags(args),
        Command::GenerateFunctionsCatalog(args) => cmd_generate_functions_catalog(args),
        Command::ExtractFunctions(args) => cmd_extract_functions(args),
        Command::AnalyzeVersions(args) => cmd_analyze_versions(args),
        Command::Lemon { args } => syntaqlite_buildtools::run_lemon(args),
        Command::Mkkeyword { args } => syntaqlite_buildtools::run_mkkeyword(args),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

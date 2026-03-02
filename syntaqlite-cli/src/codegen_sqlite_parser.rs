// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fs;
use std::path::Path;

use clap::Parser;

/// Generate internal Rust artifacts for the SQLite parser crate.
///
/// This is a flat (non-subcommand) command. It generates three internal-only
/// Rust artifacts from pre-existing inputs:
///   - functions catalog (from functions.json)
///   - ast_traits.rs (from synq + actions files via the full codegen pipeline)
///   - cflag versions table (from a pre-computed cflag audit JSON)
#[derive(Parser)]
pub(crate) struct Args {
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

pub(crate) fn dispatch(args: Args) -> Result<(), String> {
    if let Some(catalog_out) = &args.functions_catalog_out {
        let json_path = args
            .functions_json
            .as_deref()
            .ok_or("--functions-json is required when --functions-catalog-out is given")?;
        generate_functions_catalog(json_path, catalog_out)?;
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

fn generate_functions_catalog(functions_json_path: &str, output_path: &str) -> Result<(), String> {
    let json = fs::read_to_string(functions_json_path)
        .map_err(|e| format!("reading {functions_json_path}: {e}"))?;
    let content =
        syntaqlite_buildtools::util::functions_codegen::generate_functions_catalog(&json)?;
    let out = Path::new(output_path);
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("creating output directory: {e}"))?;
    }
    fs::write(out, content).map_err(|e| format!("writing {}: {e}", out.display()))?;
    eprintln!("wrote {output_path}");
    Ok(())
}

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
        "syntaqlite-parser-sqlite",
        "syntaqlite-parser",
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

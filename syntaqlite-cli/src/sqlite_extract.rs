// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fs;
use std::path::Path;

use clap::Subcommand;

/// SQLite fragment extraction CLI subcommands.
#[derive(Subcommand)]
pub(crate) enum ExtractCommand {
    /// Extract C fragments from raw SQLite source for use by the codegen pipeline.
    ///
    /// This is stage 1 of the bootstrap pipeline: it reads raw SQLite source
    /// files and produces committed fragment files that stage 3 (dialect codegen)
    /// consumes. It also vendors the SQLite tool sources and generates the
    /// base_files_tables.rs include file.
    SqliteExtract {
        /// Path to the SQLite source tree root (containing src/ and tool/).
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
    },
    /// Audit which cflags each SQLite amalgamation version references.
    ///
    /// Scans sqlite3.c for each version and writes a JSON mapping of
    /// version → recognized flag names.
    AuditCflags {
        /// Directory containing amalgamations (e.g., sqlite-amalgamations/3.35.5/sqlite3.c).
        #[arg(long, required = true)]
        amalgamation_dir: String,
        /// Output path for the audit JSON.
        #[arg(long, required = true)]
        output: String,
    },
    /// Extract built-in function catalog from pre-downloaded SQLite amalgamations.
    ///
    /// Requires a pre-computed cflag audit (from audit-cflags). Compiles each
    /// version with only the flags it supports and uses PRAGMA function_list
    /// to determine which functions are available under which conditions.
    ExtractFunctions {
        /// Directory containing amalgamations (e.g., sqlite-amalgamations/3.35.5/sqlite3.c).
        #[arg(long, required = true)]
        amalgamation_dir: String,
        /// Path to the cflag audit JSON (from audit-cflags).
        #[arg(long, required = true)]
        audit: String,
        /// Output path for the function catalog JSON.
        #[arg(long, required = true)]
        output: String,
    },
}

/// Dispatch an extraction subcommand.
pub(crate) fn dispatch(command: ExtractCommand) -> Result<(), String> {
    match command {
        ExtractCommand::SqliteExtract {
            sqlite_src,
            output_dir,
            actions_dir,
            nodes_dir,
        } => handle_sqlite_extract(&sqlite_src, &output_dir, &actions_dir, &nodes_dir),
        ExtractCommand::AuditCflags {
            amalgamation_dir,
            output,
        } => handle_audit_cflags(&amalgamation_dir, &output),
        ExtractCommand::ExtractFunctions {
            amalgamation_dir,
            audit,
            output,
        } => handle_extract_functions(&amalgamation_dir, &audit, &output),
    }
}

fn read_source(dir: &Path, name: &str) -> Result<String, String> {
    let path = dir.join(name);
    fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))
}

fn handle_sqlite_extract(
    sqlite_src: &str,
    output_dir: &str,
    actions_dir: &str,
    nodes_dir: &str,
) -> Result<(), String> {
    use syntaqlite_buildtools::extract;

    let root = Path::new(sqlite_src);
    if !root.is_dir() {
        return Err(format!("{sqlite_src} is not a directory"));
    }

    let src = root.join("src");
    let tool_dir = root.join("tool");

    let out = Path::new(output_dir);
    let sources_dir = out.join("sources");
    let fragments_dir = sources_dir.join("fragments");
    let data_dir = out.join("data");

    fs::create_dir_all(&sources_dir)
        .map_err(|e| format!("creating {}: {e}", sources_dir.display()))?;
    fs::create_dir_all(&fragments_dir)
        .map_err(|e| format!("creating {}: {e}", fragments_dir.display()))?;
    fs::create_dir_all(&data_dir).map_err(|e| format!("creating {}: {e}", data_dir.display()))?;

    // Step 1: Vendor sources — copy lemon.c, lempar.c, mkkeywordhash.c
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
        eprintln!("  {}", filename);
    }

    // Step 2: Transform mkkeywordhash.c → mkkeywordhash_modified.c
    eprintln!("Transforming mkkeywordhash.c...");
    let mkkeywordhash_c = read_source(&tool_dir, "mkkeywordhash.c")?;
    extract::mkkeywordhash::write_modified_mkkeywordhash(
        &mkkeywordhash_c,
        &sources_dir.join("mkkeywordhash_modified.c"),
    )?;

    // Step 3: Extract tokenizer fragments
    eprintln!("Extracting tokenizer fragments...");
    let tokenize_c = read_source(&src, "tokenize.c")?;
    let global_c = read_source(&src, "global.c")?;
    let sqliteint_h = read_source(&src, "sqliteInt.h")?;

    let fragments = extract::tokenizer::extract_fragments(&tokenize_c, &global_c, &sqliteint_h)?;
    extract::tokenizer::write_fragments(&fragments, &fragments_dir)?;

    // Step 4: Extract keyword cflags
    eprintln!("Extracting keyword cflag data...");
    let cflags = extract::keywords_and_parser::extract_keyword_cflags(&mkkeywordhash_c)?;
    extract::keywords_and_parser::write_keyword_cflags(
        &cflags,
        &data_dir.join("keyword_cflags.json"),
    )?;

    // Step 5: Generate base_files_tables.rs
    eprintln!("Generating base_files_tables.rs...");
    let codegen_crate_dir = Path::new(output_dir)
        .parent()
        .ok_or("output_dir has no parent")?;
    let tables_path = codegen_crate_dir.join("src/base_files_tables.rs");
    extract::base_files::write_base_files_tables(
        Path::new(actions_dir),
        Path::new(nodes_dir),
        codegen_crate_dir,
        &tables_path,
    )?;

    eprintln!("Stage 1 complete. Output: {}", out.display());
    Ok(())
}

fn handle_audit_cflags(amalgamation_dir: &str, output: &str) -> Result<(), String> {
    use syntaqlite_buildtools::extract;

    let amal_path = Path::new(amalgamation_dir);
    if !amal_path.is_dir() {
        return Err(format!("{amalgamation_dir} is not a directory"));
    }

    extract::functions::audit_version_cflags(amal_path, Path::new(output))?;
    Ok(())
}

fn handle_extract_functions(
    amalgamation_dir: &str,
    audit: &str,
    output: &str,
) -> Result<(), String> {
    use syntaqlite_buildtools::extract;

    let amal_path = Path::new(amalgamation_dir);
    if !amal_path.is_dir() {
        return Err(format!("{amalgamation_dir} is not a directory"));
    }

    extract::functions::extract_function_catalog(amal_path, Path::new(audit), Path::new(output))?;
    Ok(())
}

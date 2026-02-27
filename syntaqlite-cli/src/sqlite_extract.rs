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
        /// Path to the SQLite source directory (containing tokenize.c, etc.).
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

    let src = Path::new(sqlite_src);
    if !src.is_dir() {
        return Err(format!("{sqlite_src} is not a directory"));
    }

    let out = Path::new(output_dir);
    let sources_dir = out.join("sources");
    let fragments_dir = sources_dir.join("fragments");
    let data_dir = out.join("data");

    fs::create_dir_all(&sources_dir)
        .map_err(|e| format!("creating {}: {e}", sources_dir.display()))?;
    fs::create_dir_all(&fragments_dir)
        .map_err(|e| format!("creating {}: {e}", fragments_dir.display()))?;
    fs::create_dir_all(&data_dir)
        .map_err(|e| format!("creating {}: {e}", data_dir.display()))?;

    // mkkeywordhash.c lives in tool/, not src/
    let tool_dir = src.parent().unwrap_or(src).join("tool");

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
    let tokenize_c = read_source(src, "tokenize.c")?;
    let global_c = read_source(src, "global.c")?;
    let sqliteint_h = read_source(src, "sqliteInt.h")?;

    let fragments = extract::tokenizer::extract_fragments(&tokenize_c, &global_c, &sqliteint_h)?;
    extract::tokenizer::write_fragments(&fragments, &fragments_dir)?;

    // Step 4: Extract keyword cflags
    eprintln!("Extracting keyword cflag data...");
    let cflags = extract::keywords::extract_keyword_cflags(&mkkeywordhash_c)?;
    extract::keywords::write_keyword_cflags(&cflags, &data_dir.join("keyword_cflags.txt"))?;

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

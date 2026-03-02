// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! SQLite source extraction and version analysis subcommands.

use std::fs;
use std::path::Path;

use clap::Subcommand;

// ── sqlite-extract ────────────────────────────────────────────────────────────

/// SQLite fragment extraction CLI subcommands.
#[cfg(feature = "sqlite-extract")]
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
    /// Scans sqlite3.c for each version and writes a cflag-centric JSON file
    /// plus a generated Rust table for the runtime.
    AuditCflags {
        /// Directory containing amalgamations (e.g., sqlite-amalgamations/3.35.5/sqlite3.c).
        #[arg(long, required = true)]
        amalgamation_dir: String,
        /// Output path for the audit JSON.
        #[arg(long, required = true)]
        output: String,
        /// Output path for the generated Rust cflag versions table.
        #[arg(long, required = true)]
        rust_output: String,
    },
    /// Generate the Rust functions catalog module from functions.json.
    ///
    /// Reads the extracted `functions.json` and produces a Rust source file
    /// containing static function metadata and availability filtering.
    GenerateFunctionsCatalog {
        /// Path to functions.json (from extract-functions).
        #[arg(long, required = true)]
        functions_json: String,
        /// Output path for the generated Rust file.
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

#[cfg(feature = "sqlite-extract")]
pub(crate) fn dispatch_extract(command: ExtractCommand) -> Result<(), String> {
    match command {
        ExtractCommand::SqliteExtract {
            sqlite_src,
            output_dir,
            actions_dir,
            nodes_dir,
        } => handle_sqlite_extract(&sqlite_src, &output_dir, &actions_dir, &nodes_dir),
        ExtractCommand::GenerateFunctionsCatalog {
            functions_json,
            output,
        } => syntaqlite_buildtools::util::functions_codegen::write_functions_catalog_file(
            &functions_json,
            &output,
        ),
        ExtractCommand::AuditCflags {
            amalgamation_dir,
            output,
            rust_output,
        } => {
            use syntaqlite_buildtools::extract;
            let amal_path = Path::new(&amalgamation_dir);
            if !amal_path.is_dir() {
                return Err(format!("{amalgamation_dir} is not a directory"));
            }
            let availability =
                extract::functions::audit_version_cflags(amal_path, Path::new(&output))?;
            extract::functions::write_cflag_versions_rs(&availability, Path::new(&rust_output))
        }
        ExtractCommand::ExtractFunctions {
            amalgamation_dir,
            audit,
            output,
        } => {
            use syntaqlite_buildtools::extract;
            let amal_path = Path::new(&amalgamation_dir);
            if !amal_path.is_dir() {
                return Err(format!("{amalgamation_dir} is not a directory"));
            }
            extract::functions::extract_function_catalog(
                amal_path,
                Path::new(&audit),
                Path::new(&output),
            )
            .map(|_| ())
        }
    }
}

#[cfg(feature = "sqlite-extract")]
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
        eprintln!("  {}", filename);
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
    extract::keywords_and_parser::write_keyword_cflags(
        &cflags,
        &data_dir.join("keyword_cflags.json"),
    )?;

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

#[cfg(feature = "sqlite-extract")]
fn read_source(dir: &Path, name: &str) -> Result<String, String> {
    let path = dir.join(name);
    fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))
}

// ── version-analysis ──────────────────────────────────────────────────────────

/// Version analysis CLI subcommands.
#[cfg(feature = "version-analysis")]
#[derive(Subcommand)]
pub(crate) enum VersionAnalysisCommand {
    /// Analyze multiple SQLite source versions to find fragment variants.
    ///
    /// Reads pre-downloaded SQLite sources, extracts code fragments,
    /// hashes them to identify distinct variants, and writes JSON
    /// analysis to stdout plus raw variant files to the output directory.
    AnalyzeVersions {
        /// Directory containing per-version SQLite source trees.
        /// Expected layout: <dir>/3.35.0/src/tokenize.c, etc.
        #[arg(long, required = true)]
        sqlite_source_dir: String,
        /// Output directory for variant files.
        #[arg(long, required = true)]
        output_dir: String,
    },
}

#[cfg(feature = "version-analysis")]
pub(crate) fn dispatch_version_analysis(command: VersionAnalysisCommand) -> Result<(), String> {
    match command {
        VersionAnalysisCommand::AnalyzeVersions {
            sqlite_source_dir,
            output_dir,
        } => handle_analyze_versions(&sqlite_source_dir, &output_dir),
    }
}

#[cfg(feature = "version-analysis")]
fn handle_analyze_versions(sqlite_source_dir: &str, output_dir: &str) -> Result<(), String> {
    let source_dir = Path::new(sqlite_source_dir);
    let out_dir = Path::new(output_dir);

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

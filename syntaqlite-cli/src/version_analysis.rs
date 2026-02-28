// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::path::Path;

use clap::Subcommand;

/// Version analysis CLI subcommands.
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

pub(crate) fn dispatch(command: VersionAnalysisCommand) -> Result<(), String> {
    match command {
        VersionAnalysisCommand::AnalyzeVersions {
            sqlite_source_dir,
            output_dir,
        } => handle_analyze_versions(&sqlite_source_dir, &output_dir),
    }
}

fn handle_analyze_versions(sqlite_source_dir: &str, output_dir: &str) -> Result<(), String> {
    let source_dir = Path::new(sqlite_source_dir);
    let out_dir = Path::new(output_dir);

    std::fs::create_dir_all(out_dir).map_err(|e| format!("failed to create output dir: {e}"))?;

    let analysis = syntaqlite_buildtools::version_analysis::analyze_versions(source_dir, out_dir)?;

    // Serialize to JSON and print to stdout.
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

    // Write grammar report to output dir if available.
    if let Some(ref grammar) = analysis.grammar {
        let report =
            syntaqlite_buildtools::version_analysis::grammar::format_grammar_report(grammar);
        let report_path = out_dir.join("grammar_report.md");
        std::fs::write(&report_path, &report)
            .map_err(|e| format!("write {}: {e}", report_path.display()))?;
        eprintln!("Grammar report written to {}", report_path.display());
    }

    Ok(())
}


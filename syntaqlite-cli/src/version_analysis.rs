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

    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("failed to create output dir: {e}"))?;

    let analysis =
        syntaqlite_codegen::version_analysis::analyze_versions(source_dir, out_dir)?;

    // Serialize to JSON and print to stdout.
    let json = analysis_to_json(&analysis);
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
        eprintln!(
            "  {name}: {} variant(s){errors}",
            frag.variants.len()
        );
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
            syntaqlite_codegen::version_analysis::grammar::format_grammar_report(grammar);
        let report_path = out_dir.join("grammar_report.md");
        std::fs::write(&report_path, &report)
            .map_err(|e| format!("write {}: {e}", report_path.display()))?;
        eprintln!("Grammar report written to {}", report_path.display());
    }

    Ok(())
}

/// Build JSON output manually to avoid adding serde to the codegen crate.
fn analysis_to_json(
    analysis: &syntaqlite_codegen::version_analysis::VersionAnalysis,
) -> String {
    let mut out = String::new();
    out.push_str("{\n");

    // meta
    out.push_str("  \"meta\": {\n");
    out.push_str("    \"versions_processed\": [");
    let versions: Vec<String> = analysis.versions.iter().map(|v| format!("\"{v}\"")).collect();
    out.push_str(&versions.join(", "));
    out.push_str("]\n");
    out.push_str("  },\n");

    // fragments
    out.push_str("  \"fragments\": {\n");
    let frag_entries: Vec<String> = analysis
        .fragments
        .iter()
        .map(|(name, frag)| {
            let mut f = String::new();
            f.push_str(&format!("    \"{name}\": {{\n"));
            f.push_str(&format!(
                "      \"variant_count\": {},\n",
                frag.variants.len()
            ));

            // variants
            f.push_str("      \"variants\": [\n");
            let variant_entries: Vec<String> = frag
                .variants
                .iter()
                .map(|v| {
                    let vers: Vec<String> =
                        v.versions.iter().map(|ver| format!("\"{ver}\"")).collect();
                    format!(
                        "        {{\n          \"id\": \"{}\",\n          \"hash\": \"{}\",\n          \"versions\": [{}],\n          \"first\": \"{}\",\n          \"last\": \"{}\"\n        }}",
                        v.id,
                        v.hash,
                        vers.join(", "),
                        v.first(),
                        v.last()
                    )
                })
                .collect();
            f.push_str(&variant_entries.join(",\n"));
            f.push_str("\n      ],\n");

            // diffs
            f.push_str("      \"diffs\": [\n");
            let diff_entries: Vec<String> = frag
                .diffs
                .iter()
                .map(|d| {
                    format!(
                        "        {{\n          \"from\": \"{}\",\n          \"to\": \"{}\",\n          \"unified_diff\": {}\n        }}",
                        d.from_id,
                        d.to_id,
                        json_escape_string(&d.unified_diff)
                    )
                })
                .collect();
            f.push_str(&diff_entries.join(",\n"));
            f.push_str("\n      ],\n");

            // errors
            f.push_str("      \"errors\": [\n");
            let error_entries: Vec<String> = frag
                .errors
                .iter()
                .map(|(v, e)| {
                    format!(
                        "        {{\"version\": \"{v}\", \"error\": {}}}",
                        json_escape_string(e)
                    )
                })
                .collect();
            f.push_str(&error_entries.join(",\n"));
            f.push_str("\n      ]\n");

            f.push_str("    }");
            f
        })
        .collect();
    out.push_str(&frag_entries.join(",\n"));
    out.push_str("\n  },\n");

    // keywords
    out.push_str("  \"keywords\": {\n");
    out.push_str(&format!(
        "    \"total_keywords_latest\": {},\n",
        analysis.keywords.total_keywords_latest
    ));
    out.push_str("    \"additions\": [\n");
    let addition_entries: Vec<String> = analysis
        .keywords
        .additions
        .iter()
        .map(|a| {
            let kws: Vec<String> = a.added.iter().map(|k| format!("\"{k}\"")).collect();
            format!(
                "      {{\"version\": \"{}\", \"added\": [{}]}}",
                a.version,
                kws.join(", ")
            )
        })
        .collect();
    out.push_str(&addition_entries.join(",\n"));
    out.push_str("\n    ]\n");
    out.push_str("  }");

    // grammar
    if let Some(ref grammar) = analysis.grammar {
        out.push_str(",\n");
        out.push_str("  \"grammar\": {\n");

        // per_version summary
        out.push_str("    \"versions_parsed\": [\n");
        let gv_entries: Vec<String> = grammar
            .per_version
            .iter()
            .map(|(v, s)| {
                format!(
                    "      {{\"version\": \"{v}\", \"rules\": {}, \"nonterminals\": {}, \"token_decls\": {}, \"token_classes\": {}, \"fallbacks\": {}, \"precedences\": {}}}",
                    s.rule_signatures.len(),
                    s.nonterminals.len(),
                    s.token_decls.len(),
                    s.token_classes.len(),
                    s.fallbacks.len(),
                    s.precedences.len()
                )
            })
            .collect();
        out.push_str(&gv_entries.join(",\n"));
        out.push_str("\n    ],\n");

        // diffs
        out.push_str("    \"diffs\": [\n");
        let diff_entries: Vec<String> = grammar
            .diffs
            .iter()
            .map(|d| {
                let mut e = String::new();
                e.push_str(&format!(
                    "      {{\n        \"from\": \"{}\",\n        \"to\": \"{}\",\n",
                    d.from_version, d.to_version
                ));
                e.push_str(&format!(
                    "        \"rules_added\": {},\n        \"rules_removed\": {},\n",
                    d.rules_added.len(),
                    d.rules_removed.len()
                ));
                let ta: Vec<String> = d.tokens_added.iter().map(|t| format!("\"{t}\"")).collect();
                let tr: Vec<String> = d.tokens_removed.iter().map(|t| format!("\"{t}\"")).collect();
                e.push_str(&format!(
                    "        \"tokens_added\": [{}],\n        \"tokens_removed\": [{}],\n",
                    ta.join(", "),
                    tr.join(", ")
                ));
                let na: Vec<String> = d.nonterminals_added.iter().map(|t| format!("\"{t}\"")).collect();
                let nr: Vec<String> = d.nonterminals_removed.iter().map(|t| format!("\"{t}\"")).collect();
                e.push_str(&format!(
                    "        \"nonterminals_added\": [{}],\n        \"nonterminals_removed\": [{}]\n",
                    na.join(", "),
                    nr.join(", ")
                ));
                e.push_str("      }");
                e
            })
            .collect();
        out.push_str(&diff_entries.join(",\n"));
        out.push_str("\n    ],\n");

        // errors
        out.push_str("    \"errors\": [\n");
        let err_entries: Vec<String> = grammar
            .errors
            .iter()
            .map(|(v, e)| {
                format!(
                    "      {{\"version\": \"{v}\", \"error\": {}}}",
                    json_escape_string(e)
                )
            })
            .collect();
        out.push_str(&err_entries.join(",\n"));
        out.push_str("\n    ]\n");

        out.push_str("  }");
    }

    out.push_str("\n}\n");
    out
}

fn json_escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

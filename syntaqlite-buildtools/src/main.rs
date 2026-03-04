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

use clap::{Parser, Subcommand};

use syntaqlite_buildtools::commands;

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

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    let result: Result<(), String> = match &cli.command {
        Command::CodegenSqlite(args) => commands::SqliteCodegen {
            actions_dir: args.actions_dir.clone(),
            nodes_dir: args.nodes_dir.clone(),
        }
        .run(),
        Command::CodegenSqliteParser(args) => commands::SqliteParserCodegen {
            functions_json: args.functions_json.clone(),
            cflag_audit_json: args.cflag_audit_json.clone(),
            actions_dir: args.actions_dir.clone(),
            nodes_dir: args.nodes_dir.clone(),
            cflag_versions_out: args.cflag_versions_out.clone(),
        }
        .run(),
        Command::SqliteExtract(args) => commands::SqliteExtract {
            sqlite_src: args.sqlite_src.clone(),
            output_dir: args.output_dir.clone(),
            actions_dir: args.actions_dir.clone(),
            nodes_dir: args.nodes_dir.clone(),
        }
        .run(),
        Command::AuditCflags(args) => commands::AuditCflags {
            amalgamation_dir: args.amalgamation_dir.clone(),
            output: args.output.clone(),
            rust_output: args.rust_output.clone(),
        }
        .run(),
        Command::GenerateFunctionsCatalog(args) => commands::GenerateFunctionsCatalog {
            functions_json: args.functions_json.clone(),
            output: args.output.clone(),
        }
        .run(),
        Command::ExtractFunctions(args) => commands::ExtractFunctions {
            amalgamation_dir: args.amalgamation_dir.clone(),
            audit: args.audit.clone(),
            output: args.output.clone(),
        }
        .run(),
        Command::AnalyzeVersions(args) => commands::AnalyzeVersions {
            sqlite_source_dir: args.sqlite_source_dir.clone(),
            output_dir: args.output_dir.clone(),
        }
        .run(),
        Command::Lemon { args } => syntaqlite_buildtools::run_lemon(args),
        Command::Mkkeyword { args } => syntaqlite_buildtools::run_mkkeyword(args),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

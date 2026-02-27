// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fs;
use std::path::Path;

use clap::Subcommand;

/// Dialect codegen CLI subcommands.
///
/// Flattened into the top-level `Command` enum so the CLI interface is unchanged.
#[derive(Subcommand)]
pub(crate) enum CodegenCommand {
    /// Generate amalgamated dialect C sources for embedding.
    ///
    /// Base SQLite grammar and node files are embedded in the binary.
    /// When `--actions-dir` / `--nodes-dir` are provided, those extension
    /// files are merged with the base (same-name files replace the base).
    Dialect {
        /// Dialect identifier (e.g. "sqlite").
        #[arg(long, required = true)]
        name: String,
        /// Directory containing .y grammar action files (extensions only; base is embedded).
        #[arg(long)]
        actions_dir: Option<String>,
        /// Directory containing .synq node definitions (extensions only; base is embedded).
        #[arg(long)]
        nodes_dir: Option<String>,
        #[command(subcommand)]
        command: DialectCommand,
    },
    // Hidden subcommands for codegen subprocess support.
    // generate_codegen_artifacts() spawns current_exe() with these subcommands.
    // They must be present in any binary that calls the codegen pipeline.
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

#[derive(Subcommand)]
pub(crate) enum DialectCommand {
    /// Emit amalgamated C/H files.
    Csrc {
        /// Output directory for amalgamated files.
        #[arg(long, required = true)]
        output_dir: String,
    },
}

/// Dispatch a dialect codegen subcommand. Called from `run()` in lib.rs.
pub(crate) fn dispatch(command: CodegenCommand) -> Result<(), String> {
    match command {
        CodegenCommand::Dialect {
            name,
            actions_dir,
            nodes_dir,
            command,
        } => match command {
            DialectCommand::Csrc { output_dir } => cmd_generate_dialect(
                &name,
                actions_dir.as_deref(),
                nodes_dir.as_deref(),
                &output_dir,
            ),
        },
        CodegenCommand::Lemon { args } => syntaqlite_buildtools::run_lemon(&args),
        CodegenCommand::Mkkeyword { args } => syntaqlite_buildtools::run_mkkeyword(&args),
    }
}

fn cmd_generate_dialect(
    dialect: &str,
    actions_dir: Option<&str>,
    nodes_dir: Option<&str>,
    output_dir: &str,
) -> Result<(), String> {
    use syntaqlite_buildtools::amalgamate;
    use syntaqlite_buildtools::base_files;

    // Run codegen into a temp directory.
    let temp_dir = tempfile::TempDir::new().map_err(|e| format!("creating temp directory: {e}"))?;
    let temp = temp_dir.path();
    let csrc = temp.join("csrc");
    let include = temp.join("include").join(format!("syntaqlite_{dialect}"));
    fs::create_dir_all(&csrc).map_err(|e| format!("creating csrc dir: {e}"))?;
    fs::create_dir_all(&include).map_err(|e| format!("creating include dir: {e}"))?;

    // Load extension files from user dirs (if provided).
    let ext_y = match actions_dir {
        Some(dir) => syntaqlite_buildtools::read_named_files_from_dir(dir, "y")?,
        None => Vec::new(),
    };
    let ext_synq = match nodes_dir {
        Some(dir) => syntaqlite_buildtools::read_named_files_from_dir(dir, "synq")?,
        None => Vec::new(),
    };

    // Merge base + extensions.
    let merged_y = base_files::merge_file_sets(base_files::base_y_files(), &ext_y);
    let merged_synq = base_files::merge_file_sets(base_files::base_synq_files(), &ext_synq);

    codegen_to_dir_with_base(dialect, &merged_y, &merged_synq, &csrc, &include)?;

    let out = Path::new(output_dir);
    fs::create_dir_all(out).map_err(|e| format!("creating output dir: {e}"))?;
    let result = amalgamate::amalgamate_dialect(dialect, temp.as_ref())?;
    fs::write(out.join(format!("syntaqlite_{dialect}.h")), &result.header)
        .map_err(|e| format!("writing header: {e}"))?;
    fs::write(out.join(format!("syntaqlite_{dialect}.c")), &result.source)
        .map_err(|e| format!("writing source: {e}"))?;
    eprintln!("wrote {}/syntaqlite_{dialect}.{{h,c}}", out.display());
    Ok(())
}

/// Run the codegen pipeline from merged in-memory file sets.
fn codegen_to_dir_with_base(
    dialect: &str,
    y_files: &[(String, String)],
    synq_files: &[(String, String)],
    csrc_dir: &Path,
    include_dir: &Path,
) -> Result<(), String> {
    let dialect_spec = syntaqlite_buildtools::DialectNaming::new(dialect);
    let parser_prefix = dialect_spec.parser_symbol_prefix();

    // Extract extra keywords from extension .y files (terminals not in
    // the base keyword table are added to the hash). Duplicates with
    // base keywords are silently skipped by mkkeywordhash.
    let ext_y_contents: Vec<&str> = y_files
        .iter()
        .filter(|(name, _)| {
            // Only scan extension files (not base files).
            !syntaqlite_buildtools::base_files::base_y_files()
                .iter()
                .any(|(base_name, _)| *base_name == name.as_str())
        })
        .map(|(_, content)| content.as_str())
        .collect();
    let extra_keywords = syntaqlite_buildtools::extract_terminals_from_y(&ext_y_contents);

    let request = syntaqlite_buildtools::CodegenRequest {
        dialect: &dialect_spec,
        y_files,
        synq_files,
        extra_keywords: &extra_keywords,
        parser_symbol_prefix: Some(&parser_prefix),
        include_rust: false,
        crate_name: None,
    };
    let artifacts = syntaqlite_buildtools::generate_codegen_artifacts(&request)?;

    // Write token header.
    fs::write(
        include_dir.join(dialect_spec.tokens_header_name()),
        dialect_spec.guarded_tokens_header(&artifacts.parse_h),
    )
    .map_err(|e| format!("writing {dialect}_tokens.h: {e}"))?;

    // AST headers.
    fs::write(
        include_dir.join(dialect_spec.node_header_name()),
        artifacts.ast_nodes_h,
    )
    .map_err(|e| format!("writing {dialect}_node.h: {e}"))?;

    fs::write(csrc_dir.join("dialect_builder.h"), artifacts.ast_builder_h)
        .map_err(|e| format!("writing dialect_builder.h: {e}"))?;

    // Parse engine (raw Lemon output, compiled as part of dialect unit).
    fs::write(csrc_dir.join("sqlite_parse.c"), artifacts.parse_c)
        .map_err(|e| format!("writing sqlite_parse.c: {e}"))?;

    // Forward-declaration headers for parser and tokenizer.
    let parse_h = syntaqlite_buildtools::dialect_codegen::generate_parse_h(dialect);
    fs::write(csrc_dir.join("sqlite_parse.h"), parse_h)
        .map_err(|e| format!("writing sqlite_parse.h: {e}"))?;

    let tokenize_h = syntaqlite_buildtools::dialect_codegen::generate_tokenize_h(dialect);
    fs::write(csrc_dir.join("sqlite_tokenize.h"), tokenize_h)
        .map_err(|e| format!("writing sqlite_tokenize.h: {e}"))?;

    // Tokenizer + keywords.
    fs::write(csrc_dir.join("sqlite_tokenize.c"), artifacts.tokenize_c)
        .map_err(|e| format!("writing sqlite_tokenize.c: {e}"))?;
    fs::write(csrc_dir.join("sqlite_keyword.c"), artifacts.keyword_c)
        .map_err(|e| format!("writing sqlite_keyword.c: {e}"))?;

    // Metadata + formatter data.
    fs::write(csrc_dir.join("dialect_meta.h"), artifacts.dialect_meta_h)
        .map_err(|e| format!("writing dialect_meta.h: {e}"))?;

    fs::write(csrc_dir.join("dialect_fmt.h"), artifacts.dialect_fmt_h)
        .map_err(|e| format!("writing dialect_fmt.h: {e}"))?;

    fs::write(
        csrc_dir.join("dialect_tokens.h"),
        artifacts.dialect_tokens_h,
    )
    .map_err(|e| format!("writing dialect_tokens.h: {e}"))?;

    // Dialect descriptor + public API.
    fs::write(csrc_dir.join("dialect.c"), artifacts.dialect_c)
        .map_err(|e| format!("writing dialect.c: {e}"))?;

    fs::write(
        include_dir.join(dialect_spec.dialect_header_name()),
        artifacts.dialect_h,
    )
    .map_err(|e| format!("writing {dialect}.h: {e}"))?;

    fs::write(
        csrc_dir.join(dialect_spec.dialect_dispatch_header_name()),
        artifacts.dialect_dispatch_h,
    )
    .map_err(|e| format!("writing {dialect}_dialect_dispatch.h: {e}"))?;

    Ok(())
}

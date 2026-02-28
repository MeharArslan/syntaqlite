// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::path::Path;

use clap::Subcommand;

use crate::fs_util::{ensure_dir, write_file};

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
        /// Output directory for generated files.
        #[arg(long, required = true)]
        output_dir: String,
        /// Skip amalgamation and write raw C/H files instead.
        /// Use with --internal-prefix / --public-prefix / --dialect-include-dir
        /// to control the #include paths in the generated code.
        #[arg(long)]
        no_amalgamate: bool,
        /// Default path for the syntaqlite runtime header in the amalgamated output.
        /// Baked into the #ifndef SYNTAQLITE_RUNTIME_HEADER guard.
        #[arg(long, default_value = "syntaqlite_runtime.h")]
        runtime_header: String,
        /// Default path for the syntaqlite extension header in the amalgamated output.
        /// Baked into the #ifndef SYNTAQLITE_EXT_HEADER guard.
        #[arg(long, default_value = "syntaqlite_ext.h")]
        ext_header: String,
        /// Prefix for internal dialect headers (dialect_builder.h, dialect_meta.h, etc.).
        /// Only used with --no-amalgamate.
        #[arg(long, default_value = "")]
        internal_prefix: String,
        /// Prefix for public headers (syntaqlite/parser.h, syntaqlite/dialect.h, etc.).
        /// Only used with --no-amalgamate.
        #[arg(long, default_value = "")]
        public_prefix: String,
        /// Directory name for dialect public headers in #include directives.
        /// E.g. "syntaqlite_mydialect". Only used with --no-amalgamate.
        #[arg(long, default_value = "")]
        dialect_include_dir: String,
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
            DialectCommand::Csrc {
                output_dir,
                no_amalgamate,
                runtime_header,
                ext_header,
                internal_prefix,
                public_prefix,
                dialect_include_dir,
            } => {
                if no_amalgamate {
                    let includes = syntaqlite_buildtools::dialect_codegen::DialectCIncludes {
                        internal: &internal_prefix,
                        public: &public_prefix,
                        dialect_include_dir: &dialect_include_dir,
                    };
                    cmd_generate_dialect_raw(
                        &name,
                        actions_dir.as_deref(),
                        nodes_dir.as_deref(),
                        &output_dir,
                        &includes,
                    )
                } else {
                    cmd_generate_dialect(
                        &name,
                        actions_dir.as_deref(),
                        nodes_dir.as_deref(),
                        &output_dir,
                        &runtime_header,
                        &ext_header,
                    )
                }
            }
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
    runtime_header: &str,
    ext_header: &str,
) -> Result<(), String> {
    use syntaqlite_buildtools::amalgamate;
    use syntaqlite_buildtools::base_files;

    // Run codegen into a temp directory.  The includes use `csrc/` prefix
    // to match the temp dir layout so the amalgamator can resolve and
    // inline all internal headers.
    let temp_dir = tempfile::TempDir::new().map_err(|e| format!("creating temp directory: {e}"))?;
    let temp = temp_dir.path();
    let csrc = temp.join("csrc");
    let include = temp.join("include").join(format!("syntaqlite_{dialect}"));
    ensure_dir(&csrc, "csrc dir")?;
    ensure_dir(&include, "include dir")?;

    let amalg_includes = syntaqlite_buildtools::dialect_codegen::DialectCIncludes {
        internal: "csrc/",
        public: "",
        dialect_include_dir: "",
    };

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

    codegen_to_dir_with_base(
        dialect,
        &merged_y,
        &merged_synq,
        &csrc,
        &include,
        &amalg_includes,
    )?;

    let out = Path::new(output_dir);
    ensure_dir(out, "output dir")?;
    let result =
        amalgamate::amalgamate_dialect(dialect, temp, Some(runtime_header), Some(ext_header))?;
    write_file(&out.join(format!("syntaqlite_{dialect}.h")), &result.header)?;
    write_file(&out.join(format!("syntaqlite_{dialect}.c")), &result.source)?;
    eprintln!("wrote {}/syntaqlite_{dialect}.{{h,c}}", out.display());
    Ok(())
}

fn cmd_generate_dialect_raw(
    dialect: &str,
    actions_dir: Option<&str>,
    nodes_dir: Option<&str>,
    output_dir: &str,
    includes: &syntaqlite_buildtools::dialect_codegen::DialectCIncludes<'_>,
) -> Result<(), String> {
    use syntaqlite_buildtools::base_files;

    let out = Path::new(output_dir);
    let csrc = out.join("csrc");
    let include = out.join("include").join(format!("syntaqlite_{dialect}"));
    ensure_dir(&csrc, "csrc dir")?;
    ensure_dir(&include, "include dir")?;

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

    codegen_to_dir_with_base(dialect, &merged_y, &merged_synq, &csrc, &include, includes)?;
    eprintln!("wrote raw dialect files to {}", out.display());
    Ok(())
}

/// Run the codegen pipeline from merged in-memory file sets.
fn codegen_to_dir_with_base(
    dialect: &str,
    y_files: &[(String, String)],
    synq_files: &[(String, String)],
    csrc_dir: &Path,
    include_dir: &Path,
    includes: &syntaqlite_buildtools::dialect_codegen::DialectCIncludes<'_>,
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
        base_synq_files: Some(syntaqlite_buildtools::base_files::base_synq_files()),
        open_for_extension: false,
        dialect_c_includes: includes.clone(),
    };
    let artifacts = syntaqlite_buildtools::generate_codegen_artifacts(&request)?;

    // Write token header.
    write_file(
        &include_dir.join(dialect_spec.tokens_header_name()),
        dialect_spec.guarded_tokens_header(&artifacts.parse_h),
    )?;

    // AST headers.
    write_file(
        &include_dir.join(dialect_spec.node_header_name()),
        artifacts.ast_nodes_h,
    )?;
    write_file(&csrc_dir.join("dialect_builder.h"), artifacts.ast_builder_h)?;

    // Parse engine (raw Lemon output, compiled as part of dialect unit).
    write_file(&csrc_dir.join("sqlite_parse.c"), artifacts.parse_c)?;

    // Forward-declaration headers for parser and tokenizer.
    let parse_h = syntaqlite_buildtools::dialect_codegen::generate_parse_h(dialect, includes);
    write_file(&csrc_dir.join("sqlite_parse.h"), parse_h)?;

    write_file(&csrc_dir.join("sqlite_tokenize.h"), artifacts.tokenize_h)?;

    // Tokenizer + keywords.
    write_file(&csrc_dir.join("sqlite_tokenize.c"), artifacts.tokenize_c)?;
    write_file(&csrc_dir.join("sqlite_keyword.c"), artifacts.keyword_c)?;

    // Metadata + formatter data.
    write_file(&csrc_dir.join("dialect_meta.h"), artifacts.dialect_meta_h)?;
    write_file(&csrc_dir.join("dialect_fmt.h"), artifacts.dialect_fmt_h)?;
    write_file(
        &csrc_dir.join("dialect_tokens.h"),
        artifacts.dialect_tokens_h,
    )?;

    // Dialect descriptor + public API.
    write_file(&csrc_dir.join("dialect.c"), artifacts.dialect_c)?;
    write_file(
        &include_dir.join(dialect_spec.dialect_header_name()),
        artifacts.dialect_h,
    )?;
    write_file(
        &csrc_dir.join(dialect_spec.dialect_dispatch_header_name()),
        artifacts.dialect_dispatch_h,
    )?;

    Ok(())
}

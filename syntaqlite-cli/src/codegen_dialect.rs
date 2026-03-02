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
        /// Inline the runtime into the dialect amalgamation (produces a fully
        /// self-contained syntaqlite_<name>.{h,c} that needs no external runtime).
        /// Mutually exclusive with --no-amalgamate.
        #[arg(long, conflicts_with = "no_amalgamate")]
        full: bool,
        /// Default path for the syntaqlite runtime header in the amalgamated output.
        /// Baked into the #ifndef SYNTAQLITE_RUNTIME_HEADER guard.
        /// Only used in dialect-only mode (without --full).
        #[arg(long, default_value = "syntaqlite_runtime.h")]
        runtime_header: String,
        /// Default path for the syntaqlite extension header in the amalgamated output.
        /// Baked into the #ifndef SYNTAQLITE_EXT_HEADER guard.
        /// Only used in dialect-only mode (without --full).
        #[arg(long, default_value = "syntaqlite_dialect.h")]
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
    /// Emit the runtime-only amalgamation (syntaqlite_runtime.{h,c} +
    /// syntaqlite_dialect.h).  The output is dialect-independent and can
    /// be paired with any dialect-only amalgamation produced by `csrc`.
    Runtime {
        /// Output directory for generated files.
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
            DialectCommand::Csrc {
                output_dir,
                no_amalgamate,
                full,
                runtime_header,
                ext_header,
                internal_prefix: _,
                public_prefix: _,
                dialect_include_dir: _,
            } => {
                if no_amalgamate {
                    cmd_generate_dialect_raw(
                        &name,
                        actions_dir.as_deref(),
                        nodes_dir.as_deref(),
                        &output_dir,
                    )
                } else if full {
                    cmd_generate_dialect_full(
                        &name,
                        actions_dir.as_deref(),
                        nodes_dir.as_deref(),
                        &output_dir,
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
            DialectCommand::Runtime { output_dir } => cmd_generate_runtime(&output_dir),
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

    // Run codegen into a temp directory.  The includes use `csrc/` prefix
    // to match the temp dir layout so the amalgamator can resolve and
    // inline all internal headers.
    let temp_dir = tempfile::TempDir::new().map_err(|e| format!("creating temp directory: {e}"))?;
    let temp = temp_dir.path();

    let (merged_y, merged_synq) = load_extensions(actions_dir, nodes_dir)?;

    let include_dir_name = format!("syntaqlite_{dialect}");
    codegen_to_dir_with_base(&merged_y, &merged_synq, temp, dialect, &include_dir_name)?;

    let out = Path::new(output_dir);
    ensure_dir(out, "output dir")?;
    let result =
        amalgamate::amalgamate_dialect(dialect, temp, Some(runtime_header), Some(ext_header))?;
    write_file(&out.join(format!("syntaqlite_{dialect}.h")), &result.header)?;
    write_file(&out.join(format!("syntaqlite_{dialect}.c")), &result.source)?;
    eprintln!("wrote {}/syntaqlite_{dialect}.{{h,c}}", out.display());
    Ok(())
}

fn cmd_generate_dialect_full(
    dialect: &str,
    actions_dir: Option<&str>,
    nodes_dir: Option<&str>,
    output_dir: &str,
) -> Result<(), String> {
    use syntaqlite_buildtools::amalgamate;

    let temp_dir = tempfile::TempDir::new().map_err(|e| format!("creating temp directory: {e}"))?;
    let temp = temp_dir.path();

    let (merged_y, merged_synq) = load_extensions(actions_dir, nodes_dir)?;

    let include_dir_name = format!("syntaqlite_{dialect}");
    codegen_to_dir_with_base(&merged_y, &merged_synq, temp, dialect, &include_dir_name)?;

    let out = std::path::Path::new(output_dir);
    ensure_dir(out, "output dir")?;
    // Full mode: inline runtime + dialect into one self-contained file pair.
    let result = amalgamate::amalgamate_full(dialect, temp, temp)?;
    write_file(&out.join(format!("syntaqlite_{dialect}.h")), &result.header)?;
    write_file(&out.join(format!("syntaqlite_{dialect}.c")), &result.source)?;
    eprintln!(
        "wrote {}/syntaqlite_{dialect}.{{h,c}} (full)",
        out.display()
    );
    Ok(())
}

fn cmd_generate_runtime(output_dir: &str) -> Result<(), String> {
    use syntaqlite_buildtools::amalgamate;

    // Run the base SQLite codegen to get all C sources into a temp dir, then
    // extract just the runtime portion (engine + extension SPI header).
    let temp_dir = tempfile::TempDir::new().map_err(|e| format!("creating temp directory: {e}"))?;
    let temp = temp_dir.path();

    // Use the base SQLite grammar/nodes (no extensions) to populate the temp dir.
    let (merged_y, merged_synq) = load_extensions(None, None)?;
    codegen_to_dir_with_base(&merged_y, &merged_synq, temp, "sqlite", "syntaqlite_sqlite")?;

    let out = std::path::Path::new(output_dir);
    ensure_dir(out, "output dir")?;
    let result = amalgamate::amalgamate_runtime(temp)?;
    write_file(&out.join("syntaqlite_runtime.h"), &result.header)?;
    write_file(&out.join("syntaqlite_runtime.c"), &result.source)?;
    if let Some(ext) = &result.ext_header {
        write_file(&out.join("syntaqlite_dialect.h"), ext)?;
    }
    eprintln!(
        "wrote {}/syntaqlite_runtime.{{h,c}} + syntaqlite_dialect.h",
        out.display()
    );
    Ok(())
}

fn cmd_generate_dialect_raw(
    dialect: &str,
    actions_dir: Option<&str>,
    nodes_dir: Option<&str>,
    output_dir: &str,
) -> Result<(), String> {
    use syntaqlite_buildtools::codegen_api::{
        CodegenRequest, DialectNaming, extract_terminals_from_y, generate_codegen_artifacts,
    };
    use syntaqlite_buildtools::output_resolver::OutputLayout;

    let out = Path::new(output_dir);
    let (merged_y, merged_synq) = load_extensions(actions_dir, nodes_dir)?;
    let dialect_spec = DialectNaming::new(dialect);
    let parser_prefix = dialect_spec.parser_symbol_prefix();
    let ext_y_contents: Vec<&str> = merged_y
        .iter()
        .filter(|(name, _)| {
            !syntaqlite_buildtools::base_files::base_y_files()
                .iter()
                .any(|(base_name, _)| *base_name == name.as_str())
        })
        .map(|(_, content)| content.as_str())
        .collect();
    let extra_keywords = extract_terminals_from_y(&ext_y_contents);

    let layout = OutputLayout::for_external(out, dialect, &dialect_spec.include_dir_name());

    let artifacts = {
        let request = CodegenRequest {
            dialect: &dialect_spec,
            y_files: &merged_y,
            synq_files: &merged_synq,
            extra_keywords: &extra_keywords,
            parser_symbol_prefix: Some(&parser_prefix),
            include_rust: false,
            crate_name: None,
            base_synq_files: Some(syntaqlite_buildtools::base_files::base_synq_files()),
            open_for_extension: false,
            dialect_c_includes: layout.c_includes(),
            internal_wrappers: false,
        };
        generate_codegen_artifacts(&request)?
    };

    layout.write_codegen_artifacts(
        &dialect_spec,
        artifacts,
        &|dir| ensure_dir(dir, "output directory"),
        &|path, content| write_file(path, content),
    )?;

    eprintln!("wrote raw dialect files to {}", out.display());
    Ok(())
}

/// A set of named files: `(filename, content)` pairs.
type NamedFiles = Vec<(String, String)>;

/// Load extension `.y` and `.synq` files and merge them with the base file sets.
fn load_extensions(
    actions_dir: Option<&str>,
    nodes_dir: Option<&str>,
) -> Result<(NamedFiles, NamedFiles), String> {
    use syntaqlite_buildtools::base_files;

    let ext_y = match actions_dir {
        Some(dir) => syntaqlite_buildtools::codegen_api::read_named_files_from_dir(dir, "y")?,
        None => Vec::new(),
    };
    let ext_synq = match nodes_dir {
        Some(dir) => syntaqlite_buildtools::codegen_api::read_named_files_from_dir(dir, "synq")?,
        None => Vec::new(),
    };

    let merged_y = base_files::merge_file_sets(base_files::base_y_files(), &ext_y);
    let merged_synq = base_files::merge_file_sets(base_files::base_synq_files(), &ext_synq);
    Ok((merged_y, merged_synq))
}

/// Run the codegen pipeline from merged in-memory file sets into a temp directory.
fn codegen_to_dir_with_base(
    y_files: &[(String, String)],
    synq_files: &[(String, String)],
    temp_root: &Path,
    dialect_name: &str,
    include_dir_name: &str,
) -> Result<(), String> {
    use syntaqlite_buildtools::codegen_api::{
        CodegenRequest, DialectNaming, extract_terminals_from_y, generate_codegen_artifacts,
    };
    use syntaqlite_buildtools::output_resolver::OutputLayout;

    let dialect_spec = DialectNaming::new(dialect_name);
    let parser_prefix = dialect_spec.parser_symbol_prefix();
    let ext_y_contents: Vec<&str> = y_files
        .iter()
        .filter(|(name, _)| {
            !syntaqlite_buildtools::base_files::base_y_files()
                .iter()
                .any(|(base_name, _)| *base_name == name.as_str())
        })
        .map(|(_, content)| content.as_str())
        .collect();
    let extra_keywords = extract_terminals_from_y(&ext_y_contents);

    let layout = OutputLayout::for_amalg_temp(temp_root, dialect_name, include_dir_name);

    let artifacts = {
        let request = CodegenRequest {
            dialect: &dialect_spec,
            y_files,
            synq_files,
            extra_keywords: &extra_keywords,
            parser_symbol_prefix: Some(&parser_prefix),
            include_rust: false,
            crate_name: None,
            base_synq_files: Some(syntaqlite_buildtools::base_files::base_synq_files()),
            open_for_extension: false,
            dialect_c_includes: layout.c_includes(),
            internal_wrappers: false,
        };
        generate_codegen_artifacts(&request)?
    };

    layout.write_codegen_artifacts(
        &dialect_spec,
        artifacts,
        &|dir| ensure_dir(dir, "output directory"),
        &|path, content| write_file(path, content),
    )?;
    Ok(())
}

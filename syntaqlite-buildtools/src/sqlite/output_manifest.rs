// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use crate::{CodegenArtifacts, DialectNaming};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputBucket {
    /// C/H headers for `include/<dialect>/`.
    Include,
    /// C source files for the dialect `csrc/` directory.
    DialectCsrc,
    /// Generated Rust dialect modules (ast.rs).
    /// For internal crate: `src/sqlite/`. For external crates: `src/`.
    RustDialectSrc,
    /// Files that belong under `src/sqlite/` (e.g. generated catalogs).
    RustSqliteSrc,
    /// Files that belong under `src/` (e.g. `ast_traits.rs`).
    /// Only used by the internal syntaqlite crate.
    RustCrateSrc,
    /// Crate scaffolding Rust files (lib.rs, wrappers.rs) — only used by
    /// external dialect crates. The internal syntaqlite crate hand-maintains these.
    RustCrateScaffold,
    /// Files that belong in the crate root (e.g. `build.rs`, `Cargo.toml`).
    /// Only used by external dialect crates.
    CrateRoot,
    /// Low-level Rust files for `syntaqlite-sys/src/sqlite/` (ffi.rs, tokens.rs).
    /// These are the raw C-adjacent types; they live in the sys crate so that
    /// syntaqlite can re-export them without circular dependencies.
    RustParserSysSqliteSrc,
}

#[derive(Debug, Clone)]
pub struct OutputArtifact {
    pub bucket: OutputBucket,
    pub file_name: String,
    pub content: String,
}

pub fn sqlite_output_manifest(
    dialect: &DialectNaming,
    artifacts: CodegenArtifacts,
) -> Result<Vec<OutputArtifact>, String> {
    let mut out = vec![
        OutputArtifact {
            bucket: OutputBucket::Include,
            file_name: dialect.tokens_header_name(),
            content: dialect.guarded_tokens_header(&artifacts.parse_h),
        },
        OutputArtifact {
            bucket: OutputBucket::Include,
            file_name: dialect.node_header_name(),
            content: artifacts.ast_nodes_h,
        },
        OutputArtifact {
            bucket: OutputBucket::Include,
            file_name: dialect.dialect_header_name(),
            content: artifacts.dialect_h,
        },
        OutputArtifact {
            bucket: OutputBucket::DialectCsrc,
            file_name: "dialect_builder.h".to_string(),
            content: artifacts.ast_builder_h,
        },
        OutputArtifact {
            bucket: OutputBucket::DialectCsrc,
            file_name: "sqlite_parse.h".to_string(),
            content: artifacts.parse_api_h,
        },
        OutputArtifact {
            bucket: OutputBucket::DialectCsrc,
            file_name: "dialect_meta.h".to_string(),
            content: artifacts.dialect_meta_h,
        },
        OutputArtifact {
            bucket: OutputBucket::DialectCsrc,
            file_name: "dialect_fmt.h".to_string(),
            content: artifacts.dialect_fmt_h,
        },
        OutputArtifact {
            bucket: OutputBucket::DialectCsrc,
            file_name: "dialect_tokens.h".to_string(),
            content: artifacts.dialect_tokens_h,
        },
        OutputArtifact {
            bucket: OutputBucket::DialectCsrc,
            file_name: "dialect.c".to_string(),
            content: artifacts.dialect_c,
        },
        OutputArtifact {
            bucket: OutputBucket::DialectCsrc,
            file_name: dialect.dialect_dispatch_header_name(),
            content: artifacts.dialect_dispatch_h,
        },
        OutputArtifact {
            bucket: OutputBucket::DialectCsrc,
            file_name: "sqlite_tokenize.h".to_string(),
            content: artifacts.tokenize_h,
        },
        OutputArtifact {
            bucket: OutputBucket::DialectCsrc,
            file_name: "sqlite_parse.c".to_string(),
            content: artifacts.parse_c,
        },
        OutputArtifact {
            bucket: OutputBucket::DialectCsrc,
            file_name: "sqlite_tokenize.c".to_string(),
            content: artifacts.tokenize_c,
        },
        OutputArtifact {
            bucket: OutputBucket::DialectCsrc,
            file_name: "sqlite_keyword.c".to_string(),
            content: artifacts.keyword_c,
        },
        OutputArtifact {
            bucket: OutputBucket::DialectCsrc,
            file_name: "sqlite_keyword.h".to_string(),
            content: artifacts.keyword_h,
        },
    ];

    let rust = artifacts
        .rust
        .ok_or_else(|| "Missing Rust artifacts from codegen pipeline".to_string())?;
    out.push(OutputArtifact {
        bucket: OutputBucket::RustDialectSrc,
        file_name: "tokens.rs".to_string(),
        content: rust.tokens_rs,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::RustDialectSrc,
        file_name: "ffi.rs".to_string(),
        content: rust.ffi_rs,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::RustDialectSrc,
        file_name: "ast.rs".to_string(),
        content: rust.ast_rs,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::RustCrateScaffold,
        file_name: "lib.rs".to_string(),
        content: rust.lib_rs,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::RustCrateScaffold,
        file_name: "wrappers.rs".to_string(),
        content: rust.wrappers_rs,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::CrateRoot,
        file_name: "build.rs".to_string(),
        content: rust.build_rs,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::CrateRoot,
        file_name: "Cargo.toml".to_string(),
        content: rust.cargo_toml,
    });

    if let Some(functions_catalog_rs) = rust.functions_catalog_rs {
        out.push(OutputArtifact {
            bucket: OutputBucket::RustSqliteSrc,
            file_name: "functions_catalog.rs".to_string(),
            content: functions_catalog_rs,
        });
    }

    if let Some(ast_traits_rs) = rust.ast_traits_rs {
        out.push(OutputArtifact {
            bucket: OutputBucket::RustCrateSrc,
            file_name: "ast_traits.rs".to_string(),
            content: ast_traits_rs,
        });
    }

    Ok(out)
}

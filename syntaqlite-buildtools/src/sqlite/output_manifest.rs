// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use crate::{CodegenArtifacts, DialectNaming};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputBucket {
    Include,
    DialectCsrc,
    RustSrc,
    /// Files that belong in the crate root (e.g. `build.rs`).
    CrateRoot,
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
    let mut out = Vec::new();

    out.push(OutputArtifact {
        bucket: OutputBucket::Include,
        file_name: dialect.tokens_header_name(),
        content: dialect.guarded_tokens_header(&artifacts.parse_h),
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::Include,
        file_name: dialect.node_header_name(),
        content: artifacts.ast_nodes_h,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::Include,
        file_name: dialect.dialect_header_name(),
        content: artifacts.dialect_h,
    });

    out.push(OutputArtifact {
        bucket: OutputBucket::DialectCsrc,
        file_name: "dialect_builder.h".to_string(),
        content: artifacts.ast_builder_h,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::DialectCsrc,
        file_name: "sqlite_parse.h".to_string(),
        content: artifacts.parse_api_h,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::DialectCsrc,
        file_name: "dialect_meta.h".to_string(),
        content: artifacts.dialect_meta_h,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::DialectCsrc,
        file_name: "dialect_fmt.h".to_string(),
        content: artifacts.dialect_fmt_h,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::DialectCsrc,
        file_name: "dialect_tokens.h".to_string(),
        content: artifacts.dialect_tokens_h,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::DialectCsrc,
        file_name: "dialect.c".to_string(),
        content: artifacts.dialect_c,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::DialectCsrc,
        file_name: dialect.dialect_dispatch_header_name(),
        content: artifacts.dialect_dispatch_h,
    });

    out.push(OutputArtifact {
        bucket: OutputBucket::DialectCsrc,
        file_name: "sqlite_parse.c".to_string(),
        content: artifacts.parse_c,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::DialectCsrc,
        file_name: "sqlite_tokenize.c".to_string(),
        content: artifacts.tokenize_c,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::DialectCsrc,
        file_name: "sqlite_keyword.c".to_string(),
        content: artifacts.keyword_c,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::DialectCsrc,
        file_name: "sqlite_keyword.h".to_string(),
        content: artifacts.keyword_h,
    });

    let rust = artifacts
        .rust
        .ok_or_else(|| "Missing Rust artifacts from codegen pipeline".to_string())?;
    out.push(OutputArtifact {
        bucket: OutputBucket::RustSrc,
        file_name: "tokens.rs".to_string(),
        content: rust.tokens_rs,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::RustSrc,
        file_name: "ffi.rs".to_string(),
        content: rust.ffi_rs,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::RustSrc,
        file_name: "ast.rs".to_string(),
        content: rust.ast_rs,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::RustSrc,
        file_name: "lib.rs".to_string(),
        content: rust.lib_rs,
    });
    out.push(OutputArtifact {
        bucket: OutputBucket::RustSrc,
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

    Ok(out)
}

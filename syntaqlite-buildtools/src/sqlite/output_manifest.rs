// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use crate::{CodegenArtifacts, DialectNaming};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    /// Dialect-specific public C headers → `include/<dialect>/`.
    CPublicHeader,
    /// Shared C headers (e.g. tokens.h) → parser crate `include/` for SQLite;
    /// same as `CPublicHeader` for external dialects.
    CSharedHeader,
    /// C sources + internal headers → `csrc/`.
    CCsrc,
    /// Dialect-specific Rust modules (ffi.rs, tokens.rs, ast.rs) → dialect crate `src/`.
    RustDialect,
    /// Shared Rust modules (functions_catalog.rs, ast_traits.rs) → parser crate `src/`
    /// for SQLite; same as `RustDialect` for external dialects.
    RustShared,
    /// Crate scaffolding Rust files (lib.rs, wrappers.rs) — redirected for SQLite.
    RustScaffold,
    /// Files that belong in the crate root (build.rs, Cargo.toml).
    CrateRoot,
}

#[derive(Debug, Clone)]
pub struct OutputArtifact {
    pub kind: OutputKind,
    pub file_name: String,
    pub content: String,
}

pub fn output_manifest(
    dialect: &DialectNaming,
    artifacts: CodegenArtifacts,
) -> Result<Vec<OutputArtifact>, String> {
    let mut out = vec![
        OutputArtifact {
            kind: OutputKind::CPublicHeader,
            file_name: dialect.node_header_name(),
            content: artifacts.ast_nodes_h,
        },
        OutputArtifact {
            kind: OutputKind::CPublicHeader,
            file_name: dialect.dialect_header_name(),
            content: artifacts.dialect_h,
        },
        OutputArtifact {
            kind: OutputKind::CSharedHeader,
            file_name: dialect.tokens_header_name(),
            content: dialect.guarded_tokens_header(&artifacts.parse_h),
        },
        OutputArtifact {
            kind: OutputKind::CCsrc,
            file_name: "dialect_builder.h".to_string(),
            content: artifacts.ast_builder_h,
        },
        OutputArtifact {
            kind: OutputKind::CCsrc,
            file_name: "sqlite_parse.h".to_string(),
            content: artifacts.parse_api_h,
        },
        OutputArtifact {
            kind: OutputKind::CCsrc,
            file_name: "dialect_meta.h".to_string(),
            content: artifacts.dialect_meta_h,
        },
        OutputArtifact {
            kind: OutputKind::CCsrc,
            file_name: "dialect_fmt.h".to_string(),
            content: artifacts.dialect_fmt_h,
        },
        OutputArtifact {
            kind: OutputKind::CCsrc,
            file_name: "dialect_tokens.h".to_string(),
            content: artifacts.dialect_tokens_h,
        },
        OutputArtifact {
            kind: OutputKind::CCsrc,
            file_name: "dialect.c".to_string(),
            content: artifacts.dialect_c,
        },
        OutputArtifact {
            kind: OutputKind::CCsrc,
            file_name: dialect.dialect_dispatch_header_name(),
            content: artifacts.dialect_dispatch_h,
        },
        OutputArtifact {
            kind: OutputKind::CCsrc,
            file_name: "sqlite_tokenize.h".to_string(),
            content: artifacts.tokenize_h,
        },
        OutputArtifact {
            kind: OutputKind::CCsrc,
            file_name: "sqlite_parse.c".to_string(),
            content: artifacts.parse_c,
        },
        OutputArtifact {
            kind: OutputKind::CCsrc,
            file_name: "sqlite_tokenize.c".to_string(),
            content: artifacts.tokenize_c,
        },
        OutputArtifact {
            kind: OutputKind::CCsrc,
            file_name: "sqlite_keyword.c".to_string(),
            content: artifacts.keyword_c,
        },
        OutputArtifact {
            kind: OutputKind::CCsrc,
            file_name: "sqlite_keyword.h".to_string(),
            content: artifacts.keyword_h,
        },
    ];

    let rust = artifacts
        .rust
        .ok_or_else(|| "Missing Rust artifacts from codegen pipeline".to_string())?;
    out.push(OutputArtifact {
        kind: OutputKind::RustDialect,
        file_name: "tokens.rs".to_string(),
        content: rust.tokens_rs,
    });
    out.push(OutputArtifact {
        kind: OutputKind::RustDialect,
        file_name: "ffi.rs".to_string(),
        content: rust.ffi_rs,
    });
    out.push(OutputArtifact {
        kind: OutputKind::RustDialect,
        file_name: "ast.rs".to_string(),
        content: rust.ast_rs,
    });
    out.push(OutputArtifact {
        kind: OutputKind::RustScaffold,
        file_name: "lib.rs".to_string(),
        content: rust.lib_rs,
    });
    out.push(OutputArtifact {
        kind: OutputKind::RustScaffold,
        file_name: "wrappers.rs".to_string(),
        content: rust.wrappers_rs,
    });
    out.push(OutputArtifact {
        kind: OutputKind::CrateRoot,
        file_name: "build.rs".to_string(),
        content: rust.build_rs,
    });
    out.push(OutputArtifact {
        kind: OutputKind::CrateRoot,
        file_name: "Cargo.toml".to_string(),
        content: rust.cargo_toml,
    });

    if let Some(functions_catalog_rs) = rust.functions_catalog_rs {
        out.push(OutputArtifact {
            kind: OutputKind::RustShared,
            file_name: "functions_catalog.rs".to_string(),
            content: functions_catalog_rs,
        });
    }

    if let Some(ast_traits_rs) = rust.ast_traits_rs {
        out.push(OutputArtifact {
            kind: OutputKind::RustShared,
            file_name: "ast_traits.rs".to_string(),
            content: ast_traits_rs,
        });
    }

    Ok(out)
}

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::path::{Path, PathBuf};

use crate::sqlite::output_manifest::{OutputArtifact, OutputKind};

/// Maps an [`OutputKind`] + filename to a destination path, or `None` to skip.
pub trait OutputResolver {
    fn resolve(&self, kind: OutputKind, filename: &str) -> Option<PathBuf>;
}

/// For external dialects: all output in a single crate directory.
pub struct ExternalDialectResolver {
    /// `csrc/` — C sources and internal headers.
    pub csrc_dir: PathBuf,
    /// `include/<dialect>/` — C public + dialect headers.
    pub include_dir: PathBuf,
    /// `src/` — ALL Rust output (tokens, ffi, ast, lib, wrappers).
    pub rust_src_dir: PathBuf,
    /// Crate root — build.rs, Cargo.toml.
    pub crate_root: PathBuf,
}

impl OutputResolver for ExternalDialectResolver {
    fn resolve(&self, kind: OutputKind, filename: &str) -> Option<PathBuf> {
        match kind {
            // External dialects have no separate "shared" include location.
            OutputKind::CPublicHeader | OutputKind::CSharedHeader => {
                Some(self.include_dir.join(filename))
            }
            OutputKind::CCsrc => Some(self.csrc_dir.join(filename)),
            OutputKind::RustDialect | OutputKind::RustShared | OutputKind::RustScaffold => {
                Some(self.rust_src_dir.join(filename))
            }
            OutputKind::CrateRoot => Some(self.crate_root.join(filename)),
        }
    }
}

/// For the internal SQLite dialect: output split across parser + dialect crates.
pub struct SqliteDialectResolver {
    /// `syntaqlite-parser-sqlite/csrc/sqlite/`
    pub csrc_dir: PathBuf,
    /// `syntaqlite-parser-sqlite/include/<dialect>/`
    pub include_dir: PathBuf,
    /// `syntaqlite-parser/include/<dialect>/` — shared C headers (e.g. tokens.h).
    pub shared_include_dir: PathBuf,
    /// `syntaqlite-parser-sqlite/src/` — dialect-specific Rust.
    pub dialect_rust_src: PathBuf,
    /// `syntaqlite-parser/src/` — shared Rust (ast_traits.rs, functions_catalog.rs).
    pub shared_rust_src: PathBuf,
    /// Path to write `wrappers.rs` (e.g. `syntaqlite/src/sqlite/wrappers.rs`).
    /// `None` = skip (preserve hand-edited copy).
    pub wrappers_path: Option<PathBuf>,
}

impl OutputResolver for SqliteDialectResolver {
    fn resolve(&self, kind: OutputKind, filename: &str) -> Option<PathBuf> {
        match kind {
            OutputKind::CPublicHeader => Some(self.include_dir.join(filename)),
            OutputKind::CSharedHeader => Some(self.shared_include_dir.join(filename)),
            OutputKind::CCsrc => Some(self.csrc_dir.join(filename)),
            OutputKind::RustDialect => Some(self.dialect_rust_src.join(filename)),
            OutputKind::RustShared => Some(self.shared_rust_src.join(filename)),
            OutputKind::RustScaffold => {
                // Only wrappers.rs is written; lib.rs is hand-maintained.
                if filename == "wrappers.rs" {
                    self.wrappers_path.clone()
                } else {
                    None
                }
            }
            // build.rs and Cargo.toml are hand-maintained for the internal crate.
            OutputKind::CrateRoot => None,
        }
    }
}

/// Write a set of codegen artifacts using the given resolver.
///
/// `ensure_dir_fn` is called once per unique parent directory before any
/// writes into it. `write_file_fn` is called for each artifact whose
/// resolved path is `Some`.
pub fn write_artifacts(
    artifacts: Vec<OutputArtifact>,
    resolver: &impl OutputResolver,
    ensure_dir_fn: impl Fn(&Path) -> Result<(), String>,
    write_file_fn: impl Fn(&Path, &str) -> Result<(), String>,
) -> Result<(), String> {
    use std::collections::HashSet;
    let mut seen_dirs: HashSet<PathBuf> = HashSet::new();

    for artifact in artifacts {
        let Some(dest) = resolver.resolve(artifact.kind, &artifact.file_name) else {
            continue;
        };
        if let Some(dir) = dest.parent().filter(|d| seen_dirs.insert(d.to_path_buf())) {
            ensure_dir_fn(dir)?;
        }
        write_file_fn(&dest, &artifact.content)?;
    }
    Ok(())
}

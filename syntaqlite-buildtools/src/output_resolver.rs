// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::codegen_api::{CodegenArtifacts, DialectNaming};
use crate::dialect_codegen::c_dialect::DialectCIncludes;

/// A C header artifact: where to write it and what `#include "..."` string references it.
pub struct CHeader {
    /// Path relative to [`OutputLayout::root`] where the file is written. `None` = skip.
    pub write: Option<String>,
    /// Verbatim path used in `#include "..."` directives in other generated C files.
    pub include: String,
}

/// Describes where to write every generated artifact and what include paths to use.
///
/// All write paths are `Option<String>` relative to [`OutputLayout::root`]; `None` means skip.
/// For C headers, the [`CHeader`] struct bundles both the write path and the include path,
/// making the mapping between files-on-disk and `#include` directives explicit.
pub struct OutputLayout {
    /// Root directory. All write paths are relative to this.
    pub root: PathBuf,

    // ── C internal headers (csrc/) ──────────────────────────────────────────
    pub ast_builder_h: CHeader,
    pub dialect_meta_h: CHeader,
    pub dialect_fmt_h: CHeader,
    pub dialect_tokens_h: CHeader,
    pub dialect_dispatch_h: CHeader,
    pub parse_api_h: CHeader,
    pub tokenize_h: CHeader,
    pub keyword_h: CHeader,

    // ── C sources (csrc/) ───────────────────────────────────────────────────
    pub dialect_c: Option<String>,
    pub parse_c: Option<String>,
    pub tokenize_c: Option<String>,
    pub keyword_c: Option<String>,

    // ── C public headers (include/<dialect>/) ───────────────────────────────
    pub ast_nodes_h: CHeader,
    pub dialect_h: CHeader,
    /// Tokens header: the guarded `SYNTAQLITE_TK_*` defines (from lemon parse.h).
    pub tokens_h: CHeader,

    // ── Rust sources (src/) ─────────────────────────────────────────────────
    pub tokens_rs: Option<String>,
    pub ffi_rs: Option<String>,
    pub ast_rs: Option<String>,
    /// Shared AST trait definitions. Only written for the internal SQLite crate.
    pub ast_traits_rs: Option<String>,
    pub lib_rs: Option<String>,
    pub wrappers_rs: Option<String>,
    pub functions_catalog_rs: Option<String>,

    // ── Crate root ───────────────────────────────────────────────────────────
    pub build_rs: Option<String>,
    pub cargo_toml: Option<String>,
}

impl OutputLayout {
    /// Return a [`DialectCIncludes`] borrowing the include strings from this layout.
    pub fn c_includes(&self) -> DialectCIncludes<'_> {
        DialectCIncludes {
            ast_builder_h: &self.ast_builder_h.include,
            dialect_meta_h: &self.dialect_meta_h.include,
            dialect_fmt_h: &self.dialect_fmt_h.include,
            dialect_tokens_h: &self.dialect_tokens_h.include,
            parse_api_h: &self.parse_api_h.include,
            tokenize_h: &self.tokenize_h.include,
            keyword_h: &self.keyword_h.include,
            tokens_header: &self.tokens_h.include,
        }
    }

    /// Layout for the internal SQLite dialect spread across two crates in the workspace.
    ///
    /// - `root`: workspace root (e.g. `Path::new(".")`).
    /// - `dialect_crate`: crate directory name, e.g. `"syntaqlite-parser-sqlite"`.
    /// - `shared_crate`: shared crate directory name, e.g. `"syntaqlite-parser"`.
    /// - `dialect_name`: e.g. `"sqlite"`.
    /// - `include_dir_name`: subdirectory under `include/`, e.g. `"syntaqlite_sqlite"`.
    /// - `wrappers_path`: optional workspace-relative path for `wrappers.rs`.
    pub fn for_sqlite(
        root: &Path,
        dialect_crate: &str,
        shared_crate: &str,
        dialect_name: &str,
        include_dir_name: &str,
        wrappers_path: Option<&str>,
    ) -> Self {
        let dc = dialect_crate;
        let sc = shared_crate;
        let dn = dialect_name;
        let id = include_dir_name;
        let csrc = format!("{dc}/csrc/sqlite");
        // Include paths for SQLite are relative to the dialect crate root,
        // matching the -I flag in syntaqlite-parser-sqlite/build.rs (.include(&manifest_dir)).
        let ip = "csrc/sqlite/"; // internal include prefix relative to crate root
        OutputLayout {
            root: root.to_path_buf(),
            // C internal headers
            ast_builder_h: CHeader {
                write: Some(format!("{csrc}/dialect_builder.h")),
                include: format!("{ip}dialect_builder.h"),
            },
            dialect_meta_h: CHeader {
                write: Some(format!("{csrc}/dialect_meta.h")),
                include: format!("{ip}dialect_meta.h"),
            },
            dialect_fmt_h: CHeader {
                write: Some(format!("{csrc}/dialect_fmt.h")),
                include: format!("{ip}dialect_fmt.h"),
            },
            dialect_tokens_h: CHeader {
                write: Some(format!("{csrc}/dialect_tokens.h")),
                include: format!("{ip}dialect_tokens.h"),
            },
            dialect_dispatch_h: CHeader {
                write: Some(format!("{csrc}/{dn}_dialect_dispatch.h")),
                include: format!("{ip}{dn}_dialect_dispatch.h"),
            },
            parse_api_h: CHeader {
                write: Some(format!("{csrc}/sqlite_parse.h")),
                include: format!("{ip}sqlite_parse.h"),
            },
            tokenize_h: CHeader {
                write: Some(format!("{csrc}/sqlite_tokenize.h")),
                include: format!("{ip}sqlite_tokenize.h"),
            },
            keyword_h: CHeader {
                write: Some(format!("{csrc}/sqlite_keyword.h")),
                include: format!("{ip}sqlite_keyword.h"),
            },
            // C sources
            dialect_c: Some(format!("{csrc}/dialect.c")),
            parse_c: Some(format!("{csrc}/sqlite_parse.c")),
            tokenize_c: Some(format!("{csrc}/sqlite_tokenize.c")),
            keyword_c: Some(format!("{csrc}/sqlite_keyword.c")),
            // C public headers
            ast_nodes_h: CHeader {
                write: Some(format!("{dc}/include/{id}/{dn}_node.h")),
                include: format!("syntaqlite/{dn}_node.h"),
            },
            dialect_h: CHeader {
                write: Some(format!("{dc}/include/{id}/{dn}.h")),
                include: format!("syntaqlite/{dn}.h"),
            },
            // The tokens.h in syntaqlite-parser/include/syntaqlite/ is hand-maintained.
            tokens_h: CHeader {
                write: None,
                include: "syntaqlite/tokens.h".to_string(),
            },
            // Rust: dialect-specific in dialect_crate/src/, shared in shared_crate/src/
            tokens_rs: Some(format!("{dc}/src/tokens.rs")),
            ffi_rs: Some(format!("{dc}/src/ffi.rs")),
            ast_rs: Some(format!("{dc}/src/ast.rs")),
            ast_traits_rs: Some(format!("{sc}/src/ast_traits.rs")),
            lib_rs: None, // hand-maintained
            wrappers_rs: wrappers_path.map(str::to_string),
            functions_catalog_rs: None,
            // Crate root: hand-maintained for the internal crate
            build_rs: None,
            cargo_toml: None,
        }
    }

    /// Layout for an external dialect crate (all output in one directory).
    ///
    /// - `root`: the dialect crate root.
    /// - `dialect_name`: e.g. `"perfetto"`.
    /// - `include_dir_name`: subdirectory under `include/`, e.g. `"syntaqlite_perfetto"`.
    pub fn for_external(root: &Path, dialect_name: &str, include_dir_name: &str) -> Self {
        let dn = dialect_name;
        let id = include_dir_name;
        // External dialect crates compile with -I csrc/, so internal headers
        // are included by filename only (no directory prefix).
        OutputLayout {
            root: root.to_path_buf(),
            ast_builder_h: CHeader {
                write: Some("csrc/dialect_builder.h".to_string()),
                include: "dialect_builder.h".to_string(),
            },
            dialect_meta_h: CHeader {
                write: Some("csrc/dialect_meta.h".to_string()),
                include: "dialect_meta.h".to_string(),
            },
            dialect_fmt_h: CHeader {
                write: Some("csrc/dialect_fmt.h".to_string()),
                include: "dialect_fmt.h".to_string(),
            },
            dialect_tokens_h: CHeader {
                write: Some("csrc/dialect_tokens.h".to_string()),
                include: "dialect_tokens.h".to_string(),
            },
            dialect_dispatch_h: CHeader {
                write: Some(format!("csrc/{dn}_dialect_dispatch.h")),
                include: format!("{dn}_dialect_dispatch.h"),
            },
            parse_api_h: CHeader {
                write: Some("csrc/sqlite_parse.h".to_string()),
                include: "sqlite_parse.h".to_string(),
            },
            tokenize_h: CHeader {
                write: Some("csrc/sqlite_tokenize.h".to_string()),
                include: "sqlite_tokenize.h".to_string(),
            },
            keyword_h: CHeader {
                write: Some("csrc/sqlite_keyword.h".to_string()),
                include: "sqlite_keyword.h".to_string(),
            },
            dialect_c: Some("csrc/dialect.c".to_string()),
            parse_c: Some("csrc/sqlite_parse.c".to_string()),
            tokenize_c: Some("csrc/sqlite_tokenize.c".to_string()),
            keyword_c: Some("csrc/sqlite_keyword.c".to_string()),
            ast_nodes_h: CHeader {
                write: Some(format!("include/{id}/{dn}_node.h")),
                include: format!("{id}/{dn}_node.h"),
            },
            dialect_h: CHeader {
                write: Some(format!("include/{id}/{dn}.h")),
                include: format!("{id}/{dn}.h"),
            },
            tokens_h: CHeader {
                write: Some(format!("include/{id}/{dn}_tokens.h")),
                include: format!("{id}/{dn}_tokens.h"),
            },
            tokens_rs: Some("src/tokens.rs".to_string()),
            ffi_rs: Some("src/ffi.rs".to_string()),
            ast_rs: Some("src/ast.rs".to_string()),
            ast_traits_rs: None,
            lib_rs: Some("src/lib.rs".to_string()),
            wrappers_rs: Some("src/wrappers.rs".to_string()),
            functions_catalog_rs: None,
            build_rs: Some("build.rs".to_string()),
            cargo_toml: Some("Cargo.toml".to_string()),
        }
    }

    /// Layout for a temporary directory used by the amalgamation code path.
    /// Only C files are written; all Rust fields are `None`.
    ///
    /// - `root`: the temp directory.
    /// - `dialect_name`: e.g. `"sqlite"`.
    /// - `include_dir_name`: subdirectory under `include/`.
    pub fn for_amalg_temp(root: &Path, dialect_name: &str, include_dir_name: &str) -> Self {
        let dn = dialect_name;
        let id = include_dir_name;
        // Amalg temp uses "csrc/" prefix because the amalgamator resolves
        // headers relative to the temp root, and internal headers are in csrc/.
        let ip = "csrc/";
        OutputLayout {
            root: root.to_path_buf(),
            ast_builder_h: CHeader {
                write: Some("csrc/dialect_builder.h".to_string()),
                include: format!("{ip}dialect_builder.h"),
            },
            dialect_meta_h: CHeader {
                write: Some("csrc/dialect_meta.h".to_string()),
                include: format!("{ip}dialect_meta.h"),
            },
            dialect_fmt_h: CHeader {
                write: Some("csrc/dialect_fmt.h".to_string()),
                include: format!("{ip}dialect_fmt.h"),
            },
            dialect_tokens_h: CHeader {
                write: Some("csrc/dialect_tokens.h".to_string()),
                include: format!("{ip}dialect_tokens.h"),
            },
            dialect_dispatch_h: CHeader {
                write: Some(format!("csrc/{dn}_dialect_dispatch.h")),
                include: format!("{ip}{dn}_dialect_dispatch.h"),
            },
            parse_api_h: CHeader {
                write: Some("csrc/sqlite_parse.h".to_string()),
                include: format!("{ip}sqlite_parse.h"),
            },
            tokenize_h: CHeader {
                write: Some("csrc/sqlite_tokenize.h".to_string()),
                include: format!("{ip}sqlite_tokenize.h"),
            },
            keyword_h: CHeader {
                write: Some("csrc/sqlite_keyword.h".to_string()),
                include: format!("{ip}sqlite_keyword.h"),
            },
            dialect_c: Some("csrc/dialect.c".to_string()),
            parse_c: Some("csrc/sqlite_parse.c".to_string()),
            tokenize_c: Some("csrc/sqlite_tokenize.c".to_string()),
            keyword_c: Some("csrc/sqlite_keyword.c".to_string()),
            ast_nodes_h: CHeader {
                write: Some(format!("include/{id}/{dn}_node.h")),
                include: format!("{id}/{dn}_node.h"),
            },
            dialect_h: CHeader {
                write: Some(format!("include/{id}/{dn}.h")),
                include: format!("{id}/{dn}.h"),
            },
            tokens_h: CHeader {
                write: Some(format!("include/{id}/{dn}_tokens.h")),
                include: format!("{id}/{dn}_tokens.h"),
            },
            tokens_rs: None,
            ffi_rs: None,
            ast_rs: None,
            ast_traits_rs: None,
            lib_rs: None,
            wrappers_rs: None,
            functions_catalog_rs: None,
            build_rs: None,
            cargo_toml: None,
        }
    }

    /// Write all generated artifacts to the filesystem using this layout.
    pub fn write_codegen_artifacts(
        &self,
        dialect: &DialectNaming,
        artifacts: CodegenArtifacts,
        ensure_dir_fn: &impl Fn(&Path) -> Result<(), String>,
        write_file_fn: &impl Fn(&Path, &str) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut seen_dirs: HashSet<PathBuf> = HashSet::new();

        let mut write = |path: &Option<String>, content: &str| -> Result<(), String> {
            let Some(rel) = path else {
                return Ok(());
            };
            let dest = self.root.join(rel);
            if let Some(dir) = dest.parent()
                && seen_dirs.insert(dir.to_path_buf())
            {
                ensure_dir_fn(dir)?;
            }
            write_file_fn(&dest, content)
        };

        // C internal headers
        write(&self.ast_builder_h.write, &artifacts.ast_builder_h)?;
        write(&self.dialect_meta_h.write, &artifacts.dialect_meta_h)?;
        write(&self.dialect_fmt_h.write, &artifacts.dialect_fmt_h)?;
        write(&self.dialect_tokens_h.write, &artifacts.dialect_tokens_h)?;
        write(
            &self.dialect_dispatch_h.write,
            &artifacts.dialect_dispatch_h,
        )?;
        write(&self.parse_api_h.write, &artifacts.parse_api_h)?;
        write(&self.tokenize_h.write, &artifacts.tokenize_h)?;
        write(&self.keyword_h.write, &artifacts.keyword_h)?;

        // C sources
        write(&self.dialect_c, &artifacts.dialect_c)?;
        write(&self.parse_c, &artifacts.parse_c)?;
        write(&self.tokenize_c, &artifacts.tokenize_c)?;
        write(&self.keyword_c, &artifacts.keyword_c)?;

        // C public headers
        write(&self.ast_nodes_h.write, &artifacts.ast_nodes_h)?;
        write(&self.dialect_h.write, &artifacts.dialect_h)?;
        if self.tokens_h.write.is_some() {
            let guarded = dialect.guarded_tokens_header(&artifacts.parse_h);
            write(&self.tokens_h.write, &guarded)?;
        }

        // Rust
        if let Some(rust) = artifacts.rust {
            write(&self.tokens_rs, &rust.tokens_rs)?;
            write(&self.ffi_rs, &rust.ffi_rs)?;
            write(&self.ast_rs, &rust.ast_rs)?;
            if let Some(ref content) = rust.ast_traits_rs {
                write(&self.ast_traits_rs, content)?;
            }
            write(&self.lib_rs, &rust.lib_rs)?;
            write(&self.wrappers_rs, &rust.wrappers_rs)?;
            if let Some(ref content) = rust.functions_catalog_rs {
                write(&self.functions_catalog_rs, content)?;
            }
            write(&self.build_rs, &rust.build_rs)?;
            write(&self.cargo_toml, &rust.cargo_toml)?;
        }

        Ok(())
    }
}

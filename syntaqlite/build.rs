// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let csrc = manifest_dir.join("csrc");
    let runtime_dir = manifest_dir.join("../syntaqlite-runtime");
    let runtime_csrc = runtime_dir.join("csrc");

    // Engine sources (from runtime crate) — generic parser/tokenizer engine.
    cc::Build::new()
        .file(runtime_csrc.join("arena.c"))
        .file(runtime_csrc.join("tokenizer.c"))
        .file(runtime_csrc.join("parser.c"))
        .include(&runtime_dir) // for csrc/arena.h, csrc/parser.h, etc.
        .include(runtime_dir.join("include")) // for syntaqlite/config.h, types.h, parser.h
        .include(&manifest_dir) // for dialect csrc/ headers
        .include(manifest_dir.join("include")) // for dialect include/ headers
        .compile("syntaqlite_engine");

    // Dialect sources — Lemon parser, tokenizer, keyword lookup, and dialect glue.
    // All dialect-specific C code lives here; the engine above is grammar-agnostic.
    cc::Build::new()
        .file(csrc.join("dialect.c"))
        .file(csrc.join("sqlite_parse.c"))
        .file(csrc.join("sqlite_tokenize.c"))
        .file(csrc.join("sqlite_keyword.c"))
        .include(&runtime_dir)
        .include(runtime_dir.join("include"))
        .include(&manifest_dir)
        .include(manifest_dir.join("include"))
        .flag("-Wno-int-conversion")
        .flag("-Wno-void-pointer-to-int-cast")
        .flag("-Wno-unused-variable")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-comment")
        .compile("syntaqlite_dialect");

    // Engine (runtime) sources
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/csrc/arena.c");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include/syntaqlite_ext/arena.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include/syntaqlite_ext/vec.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include/syntaqlite_ext/ast_builder.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/csrc/parser.c");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/csrc/parse_ctx.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/csrc/tokenizer.c");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/csrc/dialect_dispatch.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include/syntaqlite_ext/sqlite_compat.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include/syntaqlite/config.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include/syntaqlite/parser.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include/syntaqlite/tokenizer.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include/syntaqlite/types.h");

    // Dialect sources
    println!("cargo:rerun-if-changed=csrc/dialect.c");
    println!("cargo:rerun-if-changed=csrc/sqlite_parse.c");
    println!("cargo:rerun-if-changed=csrc/sqlite_parse.h");
    println!("cargo:rerun-if-changed=csrc/sqlite_tokenize.c");
    println!("cargo:rerun-if-changed=csrc/sqlite_tokenize.h");
    println!("cargo:rerun-if-changed=csrc/sqlite_keyword.c");
    println!("cargo:rerun-if-changed=csrc/dialect_meta.h");
    println!("cargo:rerun-if-changed=csrc/dialect_fmt.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include/syntaqlite/dialect.h");
    println!("cargo:rerun-if-changed=include/syntaqlite_sqlite/sqlite_tokens.h");
    println!("cargo:rerun-if-changed=include/syntaqlite_sqlite/sqlite_node.h");
    println!("cargo:rerun-if-changed=include/syntaqlite_sqlite/sqlite.h");
}

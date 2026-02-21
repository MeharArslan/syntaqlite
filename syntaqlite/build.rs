// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let csrc = manifest_dir.join("csrc");
    let runtime_include = manifest_dir.join("../syntaqlite-runtime/include");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // Dialect sources — Lemon parser, tokenizer, keyword lookup, and dialect glue.
    // Grammar-agnostic engine C is built by the syntaqlite-runtime crate.
    let mut build = cc::Build::new();
    build
        .file(csrc.join("dialect.c"))
        .file(csrc.join("sqlite_parse.c"))
        .file(csrc.join("sqlite_tokenize.c"))
        .file(csrc.join("sqlite_keyword.c"))
        .include(&manifest_dir) // for dialect csrc/ headers
        .include(manifest_dir.join("include")) // for dialect include/ headers
        .include(runtime_include) // for shared syntaqlite/*.h and syntaqlite_ext/*.h
        .flag("-Wno-int-conversion")
        .flag("-Wno-void-pointer-to-int-cast")
        .flag("-Wno-unused-variable")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-comment");
    if target_os == "emscripten" {
        build.flag("-fPIC");
    }
    build.compile("syntaqlite_dialect");

    // Dialect sources
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=csrc/dialect.c");
    println!("cargo:rerun-if-changed=csrc/sqlite_parse.c");
    println!("cargo:rerun-if-changed=csrc/sqlite_parse.h");
    println!("cargo:rerun-if-changed=csrc/sqlite_tokenize.c");
    println!("cargo:rerun-if-changed=csrc/sqlite_tokenize.h");
    println!("cargo:rerun-if-changed=csrc/sqlite_keyword.c");
    println!("cargo:rerun-if-changed=csrc/dialect_meta.h");
    println!("cargo:rerun-if-changed=csrc/dialect_fmt.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include/syntaqlite/config.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include/syntaqlite/dialect.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include/syntaqlite/parser.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include/syntaqlite/types.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include/syntaqlite_ext/arena.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include/syntaqlite_ext/ast_builder.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include/syntaqlite_ext/sqlite_compat.h");
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include/syntaqlite_ext/vec.h");
    println!("cargo:rerun-if-changed=include/syntaqlite_sqlite/sqlite_tokens.h");
    println!("cargo:rerun-if-changed=include/syntaqlite_sqlite/sqlite_node.h");
    println!("cargo:rerun-if-changed=include/syntaqlite_sqlite/sqlite.h");
}

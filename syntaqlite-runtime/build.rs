// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let csrc = manifest_dir.join("csrc");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // Grammar-agnostic runtime engine sources.
    let mut build = cc::Build::new();
    build
        .file(csrc.join("tokenizer.c"))
        .file(csrc.join("parser.c"))
        .include(&manifest_dir) // for csrc/*.h internal headers
        .include(manifest_dir.join("include")); // for public syntaqlite/*.h and syntaqlite_ext/*.h
    if target_os == "emscripten" {
        build.flag("-fPIC");
    }
    build.compile("syntaqlite_engine");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=csrc");
    println!("cargo:rerun-if-changed=include");
}

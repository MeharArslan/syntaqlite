// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Minimal build script: compile vendored C tools into the binary.
//!
use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
    let sources_dir = PathBuf::from(&manifest_dir).join("sqlite-vendored/sources");

    // Compile lemon.c (parser generator tool, called as subprocess).
    cc::Build::new()
        .file(sources_dir.join("lemon.c"))
        .define("main", "lemon_main")
        .compile("lemon");

    // Compile pre-transformed mkkeywordhash.c (keyword hash tool).
    // The transformation is done by `sqlite-extract` and committed to sqlite-vendored/sources/.
    cc::Build::new()
        .file(sources_dir.join("mkkeywordhash_modified.c"))
        .define("SQLITE_ENABLE_ORDERED_SET_AGGREGATES", None)
        .flag_if_supported("-Wno-missing-field-initializers")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-variable")
        .flag_if_supported("-Wno-sign-compare")
        .compile("mkkeywordhash");

    println!("cargo:rerun-if-changed=sqlite-vendored/sources/lemon.c");
    println!("cargo:rerun-if-changed=sqlite-vendored/sources/mkkeywordhash_modified.c");
}

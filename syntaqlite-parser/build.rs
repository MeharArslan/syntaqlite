// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let csrc = manifest_dir.join("csrc");
    let include_dir = manifest_dir.join("include");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // Expose include directory so downstream crates can find headers.
    println!("cargo:include={}", include_dir.display());

    if env::var("CARGO_FEATURE_NO_BUNDLED_PARSER").is_ok() {
        // Skip C compilation — caller links a pre-built library.
        return;
    }

    // ── Grammar-agnostic engine sources ─────────────────────────────────
    //
    // Always compiled: tokenizer.c, parser.c (the runtime engine).
    let mut engine_build = cc::Build::new();
    engine_build
        .file(csrc.join("tokenizer.c"))
        .file(csrc.join("parser.c"))
        .file(csrc.join("token_wrapped.c"))
        .include(&manifest_dir) // for csrc/*.h internal headers
        .include(&include_dir); // for public syntaqlite/*.h headers (incl. sqlite_tokens.h)
    if target_os == "emscripten" {
        engine_build.flag("-fPIC");
    }
    engine_build.compile("syntaqlite_engine");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=csrc");
    println!("cargo:rerun-if-changed=include");
}

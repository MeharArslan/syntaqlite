// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let csrc = manifest_dir.join("csrc");
    let runtime_include = manifest_dir.join("../syntaqlite-runtime/include");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
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
        .include(&runtime_include) // for shared syntaqlite/*.h and syntaqlite_ext/*.h
        .flag("-Wno-int-conversion")
        .flag("-Wno-void-pointer-to-int-cast")
        .flag("-Wno-unused-variable")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-comment");
    if target_os == "emscripten" {
        build.flag("-fPIC");
    }

    // ── Version pinning ─────────────────────────────────────────────────
    //
    // With --features pin-version, reads SYNTAQLITE_SQLITE_VERSION env var
    // and passes -DSYNTAQLITE_SQLITE_VERSION=<value> to cc. This makes
    // SYNQ_VER_LT() a compile-time constant, enabling dead-branch elimination.
    //
    // Pure C users can pass the define directly:
    //   cc -DSYNTAQLITE_SQLITE_VERSION=3035000 ...
    let pinned_version = if env::var("CARGO_FEATURE_PIN_VERSION").is_ok() {
        let ver_str = env::var("SYNTAQLITE_SQLITE_VERSION").unwrap_or_else(|_| {
            panic!(
                "pin-version feature requires SYNTAQLITE_SQLITE_VERSION env var \
                 (e.g. SYNTAQLITE_SQLITE_VERSION=3035000)"
            )
        });
        let ver: i32 = ver_str.parse().unwrap_or_else(|_| {
            panic!("SYNTAQLITE_SQLITE_VERSION must be an integer (e.g. 3035000), got: {ver_str}")
        });
        build.define("SYNTAQLITE_SQLITE_VERSION", ver_str.as_str());
        Some(ver)
    } else {
        None
    };

    // ── Cflag pinning ───────────────────────────────────────────────────
    //
    // With --features pin-cflags, scans for SYNTAQLITE_CFLAG_* env vars
    // and passes the same -D flags to cc. This makes SYNQ_HAS_CFLAG() a
    // compile-time constant.
    //
    // Usage:
    //   SYNTAQLITE_CFLAG_OMIT_WINDOWFUNC=1 \
    //   SYNTAQLITE_CFLAG_ENABLE_FTS5=1 \
    //   cargo build --features pin-cflags
    //
    // Pure C users can pass the same defines directly:
    //   cc -DSYNTAQLITE_SQLITE_CFLAGS -DSYNTAQLITE_CFLAG_OMIT_WINDOWFUNC ...
    let pinned_cflag_indices: Vec<u32> = if env::var("CARGO_FEATURE_PIN_CFLAGS").is_ok() {
        let all_entries = syntaqlite_runtime::build_util::cflag_entries();
        let mut indices = Vec::new();

        // Pass the master switch.
        build.define("SYNTAQLITE_SQLITE_CFLAGS", None);

        // Scan env vars for SYNTAQLITE_CFLAG_* and pass matching -D flags.
        for entry in &all_entries {
            let env_key = format!("SYNTAQLITE_CFLAG_{}", entry.suffix);
            if env::var(&env_key).is_ok() {
                build.define(&env_key, None);
                indices.push(entry.index);
                println!("cargo:rerun-if-env-changed={env_key}");
            }
        }

        indices
    } else {
        Vec::new()
    };

    build.compile("syntaqlite_dialect");

    // ── Generate Rust pinned config ─────────────────────────────────────
    //
    // Always generate pinned_config.rs (defaults to DialectConfig::default()
    // when no pinning features are enabled).
    let rs = syntaqlite_runtime::build_util::generate_pinned_config_rs(
        pinned_version,
        &pinned_cflag_indices,
    );
    std::fs::write(out_dir.join("pinned_config.rs"), &rs).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=csrc");
    println!("cargo:rerun-if-changed=include");
    // Dialect C files #include runtime headers.
    println!("cargo:rerun-if-changed=../syntaqlite-runtime/include");
    // Re-run when pinning env vars change.
    println!("cargo:rerun-if-env-changed=SYNTAQLITE_SQLITE_VERSION");
}

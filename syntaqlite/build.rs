// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let csrc = manifest_dir.join("csrc");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // ── Grammar-agnostic engine sources ─────────────────────────────────
    //
    // Always compiled: tokenizer.c, parser.c (the runtime engine).
    let mut engine_build = cc::Build::new();
    engine_build
        .file(csrc.join("tokenizer.c"))
        .file(csrc.join("parser.c"))
        .include(&manifest_dir) // for csrc/*.h internal headers
        .include(manifest_dir.join("include")); // for public syntaqlite/*.h and syntaqlite_ext/*.h
    if target_os == "emscripten" {
        engine_build.flag("-fPIC");
    }
    engine_build.compile("syntaqlite_engine");

    // ── SQLite dialect sources (feature="sqlite") ───────────────────────
    //
    // Lemon parser, tokenizer, keyword lookup, and dialect glue.
    if env::var("CARGO_FEATURE_SQLITE").is_ok() {
        let mut build = cc::Build::new();
        build
            .file(csrc.join("dialect.c"))
            .file(csrc.join("sqlite_parse.c"))
            .file(csrc.join("sqlite_tokenize.c"))
            .file(csrc.join("sqlite_tokenize_version_gate.c"))
            .file(csrc.join("sqlite_keyword.c"))
            .include(&manifest_dir) // for dialect csrc/ headers
            .include(manifest_dir.join("include")) // for dialect include/ headers
            .flag("-Wno-int-conversion")
            .flag("-Wno-void-pointer-to-int-cast")
            .flag("-Wno-unused-variable")
            .flag("-Wno-unused-parameter")
            .flag("-Wno-comment");
        if target_os == "emscripten" {
            build.flag("-fPIC");
        }

        // ── Version pinning ─────────────────────────────────────────
        if env::var("CARGO_FEATURE_PIN_VERSION").is_ok() {
            let ver_str = env::var("SYNTAQLITE_SQLITE_VERSION").unwrap_or_else(|_| {
                panic!(
                    "pin-version feature requires SYNTAQLITE_SQLITE_VERSION env var \
                     (e.g. SYNTAQLITE_SQLITE_VERSION=3035000)"
                )
            });
            let _: i32 = ver_str.parse().unwrap_or_else(|_| {
                panic!(
                    "SYNTAQLITE_SQLITE_VERSION must be an integer (e.g. 3035000), got: {ver_str}"
                )
            });
            build.define("SYNTAQLITE_SQLITE_VERSION", ver_str.as_str());
        }

        // ── Cflag pinning ───────────────────────────────────────────
        if env::var("CARGO_FEATURE_PIN_CFLAGS").is_ok() {
            // Parse the cflags header for the SYNQ_CFLAG_IDX_* defines.
            let cflags_header =
                std::fs::read_to_string(manifest_dir.join("include/syntaqlite/sqlite_cflags.h"))
                    .expect("failed to read sqlite_cflags.h");

            // Pass the master switch.
            build.define("SYNTAQLITE_SQLITE_CFLAGS", None);

            // Scan env vars for SYNTAQLITE_CFLAG_* and pass matching -D flags.
            for line in cflags_header.lines() {
                let Some(rest) = line.strip_prefix("#define SYNQ_CFLAG_IDX_") else {
                    continue;
                };
                let mut parts = rest.split_whitespace();
                let Some(raw_suffix) = parts.next() else {
                    continue;
                };
                if raw_suffix == "COUNT" {
                    continue;
                }
                let suffix = format!("SQLITE_{raw_suffix}");
                let env_key = format!("SYNTAQLITE_CFLAG_{suffix}");
                if env::var(&env_key).is_ok() {
                    build.define(&env_key, None);
                    println!("cargo:rerun-if-env-changed={env_key}");
                }
            }
        }

        build.compile("syntaqlite_dialect");
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=csrc");
    println!("cargo:rerun-if-changed=include");
    // Re-run when pinning env vars change.
    println!("cargo:rerun-if-env-changed=SYNTAQLITE_SQLITE_VERSION");
}

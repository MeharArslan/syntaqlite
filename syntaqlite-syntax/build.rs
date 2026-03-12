// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Build script for `syntaqlite-syntax`.
//!
//! Compiles the C tokenizer and parser sources and links them into the crate.

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let csrc = manifest_dir.join("csrc");
    let include_dir = manifest_dir.join("include");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let sqlite_enabled = env::var("CARGO_FEATURE_SQLITE").is_ok();

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
        .include(&include_dir); // for public syntaqlite/*.h headers
    if !sqlite_enabled {
        engine_build.define("SYNTAQLITE_OMIT_SQLITE_API", None);
    }
    if target_os == "emscripten" {
        engine_build.flag("-fPIC");
    }
    engine_build.compile("syntaqlite_engine");

    // ── SQLite grammar ───────────────────────────────────────────────────
    if sqlite_enabled {
        let sqlite_csrc = csrc.join("sqlite");
        let mut build = cc::Build::new();
        build
            .file(sqlite_csrc.join("dialect.c"))
            .file(sqlite_csrc.join("sqlite_parse.c"))
            .file(sqlite_csrc.join("sqlite_tokenize.c"))
            .file(sqlite_csrc.join("sqlite_keyword.c"))
            .include(&manifest_dir) // for csrc/sqlite/*.h internal headers
            .include(&include_dir) // for public syntaqlite*/*.h headers
            .flag_if_supported("-Wno-int-conversion")
            .flag_if_supported("-Wno-void-pointer-to-int-cast")
            .flag_if_supported("-Wno-unused-variable")
            .flag_if_supported("-Wno-unused-parameter")
            .flag_if_supported("-Wno-comment");
        if target_os == "emscripten" {
            build.flag("-fPIC");
        }

        // ── Version pinning ──────────────────────────────────────────────
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

        // ── Cflag pinning ────────────────────────────────────────────────
        if env::var("CARGO_FEATURE_PIN_CFLAGS").is_ok() {
            let cflags_header = std::fs::read_to_string(include_dir.join("syntaqlite/cflags.h"))
                .expect("failed to read cflags.h from include/syntaqlite/");

            build.define("SYNTAQLITE_SQLITE_CFLAGS", None);

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

        build.compile("syntaqlite_sqlite");
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=csrc");
    println!("cargo:rerun-if-changed=include");
    println!("cargo:rerun-if-env-changed=SYNTAQLITE_SQLITE_VERSION");
}

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let csrc = manifest_dir.join("csrc");
    let include_dir = manifest_dir.join("include");
    // Engine headers (parser.h, tokenizer.h, etc.) live in syntaqlite-parser/include.
    let engine_include_dir = manifest_dir.join("../syntaqlite-parser/include");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if env::var("CARGO_FEATURE_NO_BUNDLED_PARSER").is_ok() {
        // Skip C compilation — caller links a pre-built library.
        return;
    }

    // ── SQLite dialect sources ──────────────────────────────────────────
    //
    // Lemon parser, tokenizer, keyword lookup, and dialect glue.
    let sqlite_csrc = csrc.join("sqlite");
    let mut build = cc::Build::new();
    build
        .file(sqlite_csrc.join("dialect.c"))
        .file(sqlite_csrc.join("sqlite_parse.c"))
        .file(sqlite_csrc.join("sqlite_tokenize.c"))
        .file(sqlite_csrc.join("sqlite_keyword.c"))
        .include(&manifest_dir) // for csrc/sqlite/*.h internal headers (resolved as csrc/sqlite/X.h)
        .include(&include_dir) // for public syntaqlite_sqlite/*.h headers (if any)
        .include(&engine_include_dir) // for syntaqlite/*.h engine headers (incl. tokens.h)
        .flag("-Wno-int-conversion")
        .flag("-Wno-void-pointer-to-int-cast")
        .flag("-Wno-unused-variable")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-comment");
    if target_os == "emscripten" {
        build.flag("-fPIC");
    }

    // ── Version pinning ─────────────────────────────────────────────────
    if env::var("CARGO_FEATURE_PIN_VERSION").is_ok() {
        let ver_str = env::var("SYNTAQLITE_SQLITE_VERSION").unwrap_or_else(|_| {
            panic!(
                "pin-version feature requires SYNTAQLITE_SQLITE_VERSION env var \
                 (e.g. SYNTAQLITE_SQLITE_VERSION=3035000)"
            )
        });
        let _: i32 = ver_str.parse().unwrap_or_else(|_| {
            panic!("SYNTAQLITE_SQLITE_VERSION must be an integer (e.g. 3035000), got: {ver_str}")
        });
        build.define("SYNTAQLITE_SQLITE_VERSION", ver_str.as_str());
    }

    // ── Cflag pinning ────────────────────────────────────────────────────
    if env::var("CARGO_FEATURE_PIN_CFLAGS").is_ok() {
        // Parse the cflags header for the SYNQ_CFLAG_IDX_* defines.
        // cflags.h lives in syntaqlite-parser/include/syntaqlite/.
        let cflags_header = std::fs::read_to_string(engine_include_dir.join("syntaqlite/cflags.h"))
            .expect("failed to read cflags.h from syntaqlite-parser/include/syntaqlite/");

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

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=csrc");
    println!("cargo:rerun-if-changed=include");
    println!("cargo:rerun-if-env-changed=SYNTAQLITE_SQLITE_VERSION");
}

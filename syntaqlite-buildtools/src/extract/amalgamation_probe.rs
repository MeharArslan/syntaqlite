// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Shared infrastructure for compiling and running probe programs against
//! SQLite amalgamations.
//!
//! Splits compilation into two steps:
//!   1. Compile `sqlite3.c` → `sqlite3.o` (the expensive part, ~seconds).
//!   2. Compile `probe.c` + link with `sqlite3.o` → binary (fast, ~ms).
//!
//! The `.o` file is cached in a persistent build directory keyed by
//! `(version, defines)`. On re-runs with the same amalgamation, step 1 is
//! skipped entirely.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Compile `sqlite3.c` into `sqlite3.o` with the given defines.
///
/// Skips compilation if the `.o` already exists and is newer than `sqlite3.c`.
/// Returns the path to the `.o` file.
fn compile_sqlite3_obj(
    amalgamation_dir: &Path,
    build_dir: &Path,
    defines: &[&str],
    label: &str,
) -> Result<PathBuf, String> {
    let sqlite3_c = amalgamation_dir.join("sqlite3.c");
    let obj_name = format!("sqlite3_{}.o", label.replace(['-', '.', ' '], "_"));
    let obj_path = build_dir.join(&obj_name);

    // Skip if .o exists and is newer than sqlite3.c.
    if obj_path.exists() {
        let src_mod = fs::metadata(&sqlite3_c)
            .and_then(|m| m.modified())
            .ok();
        let obj_mod = fs::metadata(&obj_path)
            .and_then(|m| m.modified())
            .ok();
        if let (Some(src_t), Some(obj_t)) = (src_mod, obj_mod) {
            if obj_t >= src_t {
                return Ok(obj_path);
            }
        }
    }

    let mut cmd = Command::new("cc");
    cmd.arg("-c")
        .arg("-o").arg(&obj_path)
        .arg(&sqlite3_c)
        .arg("-DSQLITE_INTROSPECTION_PRAGMAS")
        .arg("-DSQLITE_THREADSAFE=0")
        // Some OMIT flags produce code referencing ifdefed-out functions;
        // downgrade to warnings so the build can succeed.
        .arg("-Wno-implicit-function-declaration")
        .arg("-Wno-int-conversion")
        .arg(format!("-I{}", amalgamation_dir.display()));

    for def in defines {
        cmd.arg(def);
    }

    let output = cmd.output().map_err(|e| format!("running cc: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let short: String = stderr.lines().take(5).collect::<Vec<_>>().join("\n");
        // Clean up partial .o on failure.
        let _ = fs::remove_file(&obj_path);
        return Err(format!("compile sqlite3.o failed for {label}:\n{short}"));
    }

    Ok(obj_path)
}

/// Compile a probe C program and link it against a pre-compiled `sqlite3.o`.
///
/// Returns the path to the resulting binary.
fn link_probe(
    amalgamation_dir: &Path,
    build_dir: &Path,
    sqlite3_obj: &Path,
    probe_source: &str,
    label: &str,
) -> Result<PathBuf, String> {
    let probe_c_path = build_dir.join("probe.c");
    fs::write(&probe_c_path, probe_source)
        .map_err(|e| format!("writing probe.c: {e}"))?;

    let binary_name = format!("probe_{}", label.replace(['-', '.', ' '], "_"));
    let binary_path = build_dir.join(&binary_name);

    let mut cmd = Command::new("cc");
    cmd.arg("-o").arg(&binary_path)
        .arg(&probe_c_path)
        .arg(sqlite3_obj)
        .arg("-lm")
        .arg(format!("-I{}", amalgamation_dir.display()));

    let output = cmd.output().map_err(|e| format!("running cc: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let short: String = stderr.lines().take(5).collect::<Vec<_>>().join("\n");
        return Err(format!("link failed for {label}:\n{short}"));
    }

    Ok(binary_path)
}

/// Compile a probe program against an amalgamation and return the binary path.
///
/// Uses a two-step compile: `sqlite3.c` → `.o` (cached), then link with probe.
/// The `build_dir` should be a persistent directory so `.o` files survive
/// across runs.
pub fn compile_probe(
    amalgamation_dir: &Path,
    build_dir: &Path,
    defines: &[&str],
    probe_source: &str,
    label: &str,
) -> Result<PathBuf, String> {
    fs::create_dir_all(build_dir)
        .map_err(|e| format!("creating build dir: {e}"))?;

    let obj = compile_sqlite3_obj(amalgamation_dir, build_dir, defines, label)?;
    link_probe(amalgamation_dir, build_dir, &obj, probe_source, label)
}

/// Run a compiled probe binary and return its stdout.
pub fn run_probe(binary_path: &Path) -> Result<String, String> {
    let output = Command::new(binary_path)
        .output()
        .map_err(|e| format!("running probe: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("probe failed: {stderr}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

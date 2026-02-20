// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! C amalgamation: produces single-file compilation units from the syntaqlite
//! runtime and dialect source trees.
//!
//! Three modes:
//! - **Runtime only** — engine (`syntaqlite_runtime.{h,c}`) + extension header (`syntaqlite_ext.h`)
//! - **Dialect only** — dialect sources that `#include` the runtime header and ext header
//! - **Full** — runtime + dialect inlined into one pair of files

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct AmalgamateOutput {
    pub header: String,
    pub source: String,
    /// Extension header (present for runtime-only amalgamation).
    pub ext_header: Option<String>,
}

/// Produce `syntaqlite_runtime.{h,c}` and `syntaqlite_ext.h`.
pub fn amalgamate_runtime(runtime_dir: &Path) -> Result<AmalgamateOutput, String> {
    let csrc = runtime_dir.join("csrc");
    let include = runtime_dir.join("include");
    let graph = collect_files(&[&csrc, &include])?;
    validate_graph(&graph)?;
    emit(&graph, EmitMode::RuntimeOnly)
}

/// Produce `syntaqlite_<dialect>.{h,c}` that references `syntaqlite_runtime.h`
/// and `syntaqlite_ext.h`.
///
/// Any `#include "..."` that doesn't resolve to a file in the dialect tree is
/// assumed to be a runtime header and is stripped — the emitted `.c` file
/// includes the runtime amalgamation header via `SYNTAQLITE_RUNTIME_HEADER`
/// and the extension header via `SYNTAQLITE_EXT_HEADER`.
pub fn amalgamate_dialect(dialect: &str, dialect_dir: &Path) -> Result<AmalgamateOutput, String> {
    let csrc = dialect_dir.join("csrc");
    let include = dialect_dir.join("include");
    let graph = collect_files(&[&csrc, &include])?;
    validate_graph(&graph)?;
    emit(&graph, EmitMode::DialectOnly(dialect))
}

/// Produce `syntaqlite_<dialect>.{h,c}` with the runtime inlined.
pub fn amalgamate_full(
    dialect: &str,
    runtime_dir: &Path,
    dialect_dir: &Path,
) -> Result<AmalgamateOutput, String> {
    let dialect_csrc = dialect_dir.join("csrc");
    let dialect_include = dialect_dir.join("include");
    let runtime_csrc = runtime_dir.join("csrc");
    let runtime_include = runtime_dir.join("include");

    let graph = collect_files(&[
        &runtime_csrc,
        &runtime_include,
        &dialect_csrc,
        &dialect_include,
    ])?;
    validate_graph(&graph)?;
    emit(&graph, EmitMode::Full(dialect))
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct SourceFile {
    /// Keys in the include-path map that resolve to this file (e.g. `"csrc/arena.h"`).
    include_key: String,
    /// Classification.
    kind: FileKind,
    /// Raw file content.
    content: String,
    /// Indices of other SourceFiles this file depends on (resolved local includes).
    deps: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FileKind {
    PublicHeader,   // under include/ (key starts with `syntaqlite/`)
    ExtHeader,      // under include/ (key starts with `syntaqlite_ext/`)
    InternalHeader, // under csrc/, extension .h
    Source,         // extension .c
}

struct FileGraph {
    files: Vec<SourceFile>,
}

// ---------------------------------------------------------------------------
// Step 1: Collect files
// ---------------------------------------------------------------------------

fn collect_files(dirs: &[&Path]) -> Result<FileGraph, String> {
    let mut path_map: BTreeMap<String, PathBuf> = BTreeMap::new();

    for &dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // For `include/` directories, start with empty prefix so
        // `include/syntaqlite/foo.h` → key `syntaqlite/foo.h`.
        // `include/syntaqlite_ext/foo.h` → key `syntaqlite_ext/foo.h`.
        // For `csrc/` directories, prefix is `csrc` → key `csrc/foo.h`.
        let prefix = if dir_name == "include" { "" } else { dir_name };
        walk_dir(dir, prefix, &mut path_map)?;
    }

    // Build SourceFile entries.
    let mut files: Vec<SourceFile> = Vec::new();
    let mut key_to_idx: HashMap<String, usize> = HashMap::new();

    for (include_key, abs_path) in &path_map {
        let content = fs::read_to_string(abs_path)
            .map_err(|e| format!("reading {}: {e}", abs_path.display()))?;
        let kind = classify(include_key);
        let idx = files.len();
        key_to_idx.insert(include_key.clone(), idx);
        files.push(SourceFile {
            include_key: include_key.clone(),
            kind,
            content,
            deps: Vec::new(),
        });
    }

    // Resolve includes.
    for i in 0..files.len() {
        let local_incs = parse_local_includes(&files[i].content);
        let mut deps = Vec::new();
        for inc in local_incs {
            if let Some(&idx) = key_to_idx.get(&inc) {
                if idx != i {
                    deps.push(idx);
                }
            }
            // Unresolved local includes are simply not tracked — they'll be
            // stripped during emit (assumed to be runtime headers).
        }
        files[i].deps = deps;
    }

    Ok(FileGraph { files })
}

fn walk_dir(dir: &Path, prefix: &str, map: &mut BTreeMap<String, PathBuf>) -> Result<(), String> {
    let entries =
        fs::read_dir(dir).map_err(|e| format!("reading directory {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("reading entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            let sub_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let sub_prefix = if prefix.is_empty() {
                sub_name.to_string()
            } else {
                format!("{prefix}/{sub_name}")
            };
            walk_dir(&path, &sub_prefix, map)?;
        } else {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == "c" || ext == "h" {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let key = if prefix.is_empty() {
                    name.to_string()
                } else {
                    format!("{prefix}/{name}")
                };
                map.entry(key).or_insert_with(|| path.clone());
            }
        }
    }
    Ok(())
}

fn classify(include_key: &str) -> FileKind {
    if include_key.starts_with("syntaqlite_ext/") {
        FileKind::ExtHeader
    } else if include_key.starts_with("syntaqlite/") || include_key.starts_with("syntaqlite_") {
        // Matches runtime headers (syntaqlite/types.h) and dialect headers
        // (syntaqlite_sqlite/sqlite.h, syntaqlite_perfetto/perfetto.h, etc.).
        // syntaqlite_ext/ is handled above.
        FileKind::PublicHeader
    } else if include_key.ends_with(".h") {
        FileKind::InternalHeader
    } else {
        FileKind::Source
    }
}

// ---------------------------------------------------------------------------
// Step 2: Parse #include directives
// ---------------------------------------------------------------------------

/// Extract `#include "..."` paths from file content.
fn parse_local_includes(content: &str) -> Vec<String> {
    let mut local = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("#include") {
            let rest = rest.trim();
            if let Some(path) = rest.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
                local.push(path.to_string());
            }
        }
    }
    local
}

// ---------------------------------------------------------------------------
// Step 3: Validate graph (cycle check)
// ---------------------------------------------------------------------------

fn validate_graph(graph: &FileGraph) -> Result<(), String> {
    let n = graph.files.len();
    let mut visited = vec![0u8; n]; // 0=unvisited, 1=in-progress, 2=done
    for i in 0..n {
        if visited[i] == 0 {
            check_cycle(i, &graph.files, &mut visited)?;
        }
    }
    Ok(())
}

fn check_cycle(i: usize, files: &[SourceFile], visited: &mut [u8]) -> Result<(), String> {
    visited[i] = 1;
    for &dep in &files[i].deps {
        match visited[dep] {
            0 => check_cycle(dep, files, visited)?,
            1 => {
                return Err(format!(
                    "include cycle detected: {} -> {}",
                    files[i].include_key, files[dep].include_key
                ));
            }
            _ => {}
        }
    }
    visited[i] = 2;
    Ok(())
}

// ---------------------------------------------------------------------------
// Step 4: Topological sort
// ---------------------------------------------------------------------------

fn toposort(indices: &[usize], files: &[SourceFile]) -> Vec<usize> {
    let index_set: HashSet<usize> = indices.iter().copied().collect();
    let mut visited: HashSet<usize> = HashSet::new();
    let mut order: Vec<usize> = Vec::new();

    for &i in indices {
        toposort_visit(i, files, &index_set, &mut visited, &mut order);
    }
    order
}

fn toposort_visit(
    i: usize,
    files: &[SourceFile],
    set: &HashSet<usize>,
    visited: &mut HashSet<usize>,
    order: &mut Vec<usize>,
) {
    if !set.contains(&i) || !visited.insert(i) {
        return;
    }
    for &dep in &files[i].deps {
        toposort_visit(dep, files, set, visited, order);
    }
    order.push(i);
}

// ---------------------------------------------------------------------------
// Step 5: Emit amalgamated output
// ---------------------------------------------------------------------------

/// Amalgamation mode — determines output structure and naming.
enum EmitMode<'a> {
    /// Runtime only: `syntaqlite_runtime.{h,c}` + `syntaqlite_ext.h`.
    RuntimeOnly,
    /// Dialect only: `syntaqlite_<name>.{h,c}`, expects external runtime/ext headers.
    DialectOnly(&'a str),
    /// Full: runtime + dialect inlined into `syntaqlite_<name>.{h,c}`.
    Full(&'a str),
}

/// Emit the amalgamated header and source files.
fn emit(graph: &FileGraph, mode: EmitMode) -> Result<AmalgamateOutput, String> {
    let files = &graph.files;
    let all_indices: Vec<usize> = (0..files.len()).collect();

    // Partition by kind.
    let mut public_headers: Vec<usize> = Vec::new();
    let mut ext_headers: Vec<usize> = Vec::new();
    let mut internal_headers: Vec<usize> = Vec::new();
    let mut sources: Vec<usize> = Vec::new();

    for &i in &all_indices {
        match files[i].kind {
            FileKind::PublicHeader => public_headers.push(i),
            FileKind::ExtHeader => ext_headers.push(i),
            FileKind::InternalHeader => internal_headers.push(i),
            FileKind::Source => sources.push(i),
        }
    }

    // Toposort within each class.
    let public_headers = toposort(&public_headers, files);
    let ext_headers = toposort(&ext_headers, files);
    let internal_headers = toposort(&internal_headers, files);
    let sources = toposort(&sources, files);

    // The set of include keys being inlined (all collected files).
    let inlined_keys: HashSet<&str> = files.iter().map(|f| f.include_key.as_str()).collect();

    // Determine naming from mode.
    let (guard, header_filename) = match &mode {
        EmitMode::DialectOnly(d) | EmitMode::Full(d) => (
            format!("SYNTAQLITE_{}_H", d.to_uppercase()),
            format!("syntaqlite_{d}.h"),
        ),
        EmitMode::RuntimeOnly => (
            "SYNTAQLITE_RUNTIME_H".to_string(),
            "syntaqlite_runtime.h".to_string(),
        ),
    };

    // ── Build .h ──
    let mut header = String::new();
    header.push_str("/*\n");
    header.push_str("** syntaqlite amalgamation — machine generated, do not edit.\n");
    header.push_str("*/\n");
    header.push_str(&format!("#ifndef {guard}\n#define {guard}\n\n"));

    for &i in &public_headers {
        emit_file(&files[i], &inlined_keys, &mut header);
    }

    header.push_str(&format!("\n#endif  /* {guard} */\n"));

    // ── Build ext header (runtime-only mode) ──
    let ext_header = if matches!(mode, EmitMode::RuntimeOnly) && !ext_headers.is_empty() {
        let mut ext = String::new();
        ext.push_str("/*\n");
        ext.push_str("** syntaqlite amalgamation — machine generated, do not edit.\n");
        ext.push_str("** Extension header for dialect authors.\n");
        ext.push_str("*/\n");
        ext.push_str("#ifndef SYNTAQLITE_EXT_H\n#define SYNTAQLITE_EXT_H\n\n");
        ext.push_str("#include \"syntaqlite_runtime.h\"\n\n");

        for &i in &ext_headers {
            emit_file(&files[i], &inlined_keys, &mut ext);
        }

        ext.push_str("\n#endif  /* SYNTAQLITE_EXT_H */\n");
        Some(ext)
    } else {
        None
    };

    // ── Build .c ──
    let mut source = String::new();
    source.push_str("/*\n");
    source.push_str("** syntaqlite amalgamation — machine generated, do not edit.\n");
    source.push_str("*/\n\n");

    // Dialect-only mode: include external runtime/ext headers.
    if matches!(mode, EmitMode::DialectOnly(_)) {
        source.push_str("#ifndef SYNTAQLITE_RUNTIME_HEADER\n");
        source.push_str("#define SYNTAQLITE_RUNTIME_HEADER \"syntaqlite_runtime.h\"\n");
        source.push_str("#endif\n");
        source.push_str("#include SYNTAQLITE_RUNTIME_HEADER\n\n");
        source.push_str("#ifndef SYNTAQLITE_EXT_HEADER\n");
        source.push_str("#define SYNTAQLITE_EXT_HEADER \"syntaqlite_ext.h\"\n");
        source.push_str("#endif\n");
        source.push_str("#include SYNTAQLITE_EXT_HEADER\n\n");
    }
    source.push_str(&format!("#include \"{header_filename}\"\n\n"));

    source.push('\n');

    // Full mode: extension headers go into the .c alongside internal headers.
    if matches!(mode, EmitMode::Full(_)) {
        for &i in &ext_headers {
            emit_file(&files[i], &inlined_keys, &mut source);
        }
    }

    // Emit dialect dispatch headers before other internal headers so that
    // the direct-call macros are defined before dialect_dispatch.h is
    // processed (its fallback is guarded by `#elif !defined(SYNQ_PARSER_ALLOC)`).
    let (dispatch_headers, other_headers): (Vec<usize>, Vec<usize>) =
        internal_headers.iter().partition(|&&i| {
            files[i].include_key.ends_with("_dialect_dispatch.h")
        });
    for &i in &dispatch_headers {
        emit_file(&files[i], &inlined_keys, &mut source);
    }
    for &i in &other_headers {
        emit_file(&files[i], &inlined_keys, &mut source);
    }
    for &i in &sources {
        emit_file(&files[i], &inlined_keys, &mut source);
    }

    Ok(AmalgamateOutput {
        header,
        source,
        ext_header,
    })
}

/// Detect the include-guard macro of a header file, if any.
///
/// Returns `Some(guard)` when the first two preprocessor directives are
/// `#ifndef GUARD` / `#define GUARD` and the file ends with `#endif`.
fn detect_include_guard(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();

    // Find first two preprocessor directives (skip blanks and comments).
    let mut pp = Vec::new();
    for &line in &lines {
        let t = line.trim();
        if t.is_empty()
            || t.starts_with("//")
            || t.starts_with("/*")
            || t.starts_with("**")
            || t.starts_with("*/")
        {
            continue;
        }
        if t.starts_with('#') {
            pp.push(t);
            if pp.len() == 2 {
                break;
            }
        } else {
            // Non-preprocessor, non-blank/comment content before two directives → no guard.
            return None;
        }
    }

    if pp.len() < 2 {
        return None;
    }

    let guard = pp[0].strip_prefix("#ifndef")?.trim().to_string();
    if guard.is_empty() {
        return None;
    }
    let define_guard = pp[1].strip_prefix("#define")?.trim().to_string();
    if define_guard != guard {
        return None;
    }

    // Verify there's a trailing `#endif` (with optional comment).
    for &line in lines.iter().rev() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("#endif") {
            return Some(guard);
        }
        break;
    }
    None
}

/// Emit a single file's content, stripping all `#include "..."` directives
/// (already inlined or provided by the runtime amalgamation header) and
/// include guards (the outer amalgamation guard subsumes them).
fn emit_file(file: &SourceFile, _inlined_keys: &HashSet<&str>, out: &mut String) {
    out.push_str(&format!(
        "/* ======== begin: {} ======== */\n",
        file.include_key
    ));

    let guard = detect_include_guard(&file.content);

    // Track whether we've seen the `#ifndef GUARD` / `#define GUARD` pair
    // and whether we need to strip the final `#endif`.
    let mut guard_ifndef_seen = false;
    let mut guard_define_seen = false;
    let mut lines: Vec<&str> = file.content.lines().collect();

    // If we have a guard, strip the trailing `#endif` (search from end).
    if guard.is_some() {
        for i in (0..lines.len()).rev() {
            let t = lines[i].trim();
            if t.is_empty() {
                continue;
            }
            if t.starts_with("#endif") {
                lines[i] = "";
                break;
            }
            break;
        }
    }

    for line in &lines {
        let trimmed = line.trim();

        // Strip include guard directives.
        if let Some(ref g) = guard {
            if !guard_ifndef_seen {
                if let Some(rest) = trimmed.strip_prefix("#ifndef") {
                    if rest.trim() == g {
                        guard_ifndef_seen = true;
                        continue;
                    }
                }
            } else if !guard_define_seen {
                if let Some(rest) = trimmed.strip_prefix("#define") {
                    if rest.trim() == g {
                        guard_define_seen = true;
                        continue;
                    }
                }
            }
        }

        if let Some(rest) = trimmed.strip_prefix("#include") {
            let rest = rest.trim();
            if rest.starts_with('"') {
                // All quoted local includes are stripped:
                // - Inlined files are already present above in the amalgamation.
                // - Unresolved files are runtime headers provided by the
                //   runtime amalgamation or the self-include at the top.
                continue;
            }
            // System includes (`<...>`) are kept in place — they may be
            // inside conditional blocks (e.g. `#ifdef __cplusplus`).
            // Macro includes (e.g. `#include SYNTAQLITE_INLINE_DIALECT_DISPATCH`)
            // are also kept as-is.
        }

        out.push_str(line);
        out.push('\n');
    }

    out.push_str(&format!(
        "/* ======== end: {} ======== */\n\n",
        file.include_key
    ));
}

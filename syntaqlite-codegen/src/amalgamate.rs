// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! C amalgamation: produces single-file compilation units from the syntaqlite
//! runtime and dialect source trees.
//!
//! Three modes:
//! - **Runtime only** — just the grammar-agnostic engine (`syntaqlite_runtime.{h,c}`)
//! - **Dialect only** — dialect sources that `#include` the runtime header
//! - **Full** — runtime + dialect inlined into one pair of files

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct AmalgamateOutput {
    pub header: String,
    pub source: String,
}

/// Produce `syntaqlite_runtime.{h,c}`.
pub fn amalgamate_runtime(runtime_dir: &Path) -> Result<AmalgamateOutput, String> {
    let csrc = runtime_dir.join("csrc");
    let include = runtime_dir.join("include");
    let graph = collect_files(&[&csrc, &include])?;
    validate_graph(&graph)?;
    emit(&graph, None)
}

/// Produce `syntaqlite_<dialect>.{h,c}` that references `syntaqlite_runtime.h`.
///
/// Any `#include "..."` that doesn't resolve to a file in the dialect tree is
/// assumed to be a runtime header and is stripped — the emitted `.c` file
/// includes the runtime amalgamation header via `SYNTAQLITE_RUNTIME_HEADER`.
pub fn amalgamate_dialect(dialect: &str, dialect_dir: &Path) -> Result<AmalgamateOutput, String> {
    let csrc = dialect_dir.join("csrc");
    let include = dialect_dir.join("include");
    let graph = collect_files(&[&csrc, &include])?;
    validate_graph(&graph)?;
    emit(&graph, Some(dialect))
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
    emit(&graph, None)
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
    /// System includes (`<...>`) found in this file.
    system_includes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FileKind {
    PublicHeader,   // under include/ (key starts with `syntaqlite/`)
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
            system_includes: Vec::new(),
        });
    }

    // Resolve includes.
    for i in 0..files.len() {
        let (local_incs, sys_incs) = parse_includes(&files[i].content);
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
        files[i].system_includes = sys_incs;
    }

    Ok(FileGraph { files })
}

fn walk_dir(
    dir: &Path,
    prefix: &str,
    map: &mut BTreeMap<String, PathBuf>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("reading directory {}: {e}", dir.display()))?;
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
    if include_key.starts_with("syntaqlite/") {
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

/// Returns (local_includes, system_includes).
fn parse_includes(content: &str) -> (Vec<String>, Vec<String>) {
    let mut local = Vec::new();
    let mut system = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("#include") {
            let rest = rest.trim();
            if let Some(path) = rest.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
                local.push(path.to_string());
            } else if let Some(path) = rest.strip_prefix('<').and_then(|r| r.strip_suffix('>')) {
                system.push(format!("<{path}>"));
            }
        }
    }
    (local, system)
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

/// Emit the amalgamated header and source files.
///
/// If `dialect` is Some, the source file includes the runtime header via an
/// overridable `SYNTAQLITE_RUNTIME_HEADER` macro.
fn emit(graph: &FileGraph, dialect: Option<&str>) -> Result<AmalgamateOutput, String> {
    let files = &graph.files;
    let all_indices: Vec<usize> = (0..files.len()).collect();

    // Partition by kind.
    let mut public_headers: Vec<usize> = Vec::new();
    let mut internal_headers: Vec<usize> = Vec::new();
    let mut sources: Vec<usize> = Vec::new();

    for &i in &all_indices {
        match files[i].kind {
            FileKind::PublicHeader => public_headers.push(i),
            FileKind::InternalHeader => internal_headers.push(i),
            FileKind::Source => sources.push(i),
        }
    }

    // Toposort within each class.
    let public_headers = toposort(&public_headers, files);
    let internal_headers = toposort(&internal_headers, files);
    let sources = toposort(&sources, files);

    // Collect all system includes.
    let mut all_system_includes: BTreeSet<String> = BTreeSet::new();
    for f in files {
        for inc in &f.system_includes {
            all_system_includes.insert(inc.to_string());
        }
    }

    // The set of include keys being inlined (all collected files).
    let inlined_keys: HashSet<&str> = files.iter().map(|f| f.include_key.as_str()).collect();

    // ── Build .h ──
    let mut header = String::new();
    header.push_str("/*\n");
    header.push_str("** syntaqlite amalgamation — machine generated, do not edit.\n");
    header.push_str("*/\n");

    let guard = if let Some(d) = dialect {
        format!("SYNTAQLITE_{}_H", d.to_uppercase())
    } else if files.iter().any(|f| f.include_key.starts_with("syntaqlite/sqlite")) {
        "SYNTAQLITE_AMALGAMATION_H".to_string()
    } else {
        "SYNTAQLITE_RUNTIME_H".to_string()
    };
    header.push_str(&format!("#ifndef {guard}\n#define {guard}\n\n"));

    // System includes used by public headers.
    let pub_sys: BTreeSet<String> = public_headers
        .iter()
        .flat_map(|&i| files[i].system_includes.iter().cloned())
        .collect();
    for inc in &pub_sys {
        header.push_str(&format!("#include {inc}\n"));
    }
    if !pub_sys.is_empty() {
        header.push('\n');
    }

    for &i in &public_headers {
        emit_file(&files[i], &inlined_keys, &mut header);
    }

    header.push_str(&format!("\n#endif  /* {guard} */\n"));

    // ── Build .c ──
    let mut source = String::new();
    source.push_str("/*\n");
    source.push_str("** syntaqlite amalgamation — machine generated, do not edit.\n");
    source.push_str("*/\n\n");

    // System includes (deduped).
    for inc in &all_system_includes {
        source.push_str(&format!("#include {inc}\n"));
    }
    source.push('\n');

    // Include our own header (+ runtime header for dialect-only mode).
    if let Some(d) = dialect {
        source.push_str("#ifndef SYNTAQLITE_RUNTIME_HEADER\n");
        source.push_str("#define SYNTAQLITE_RUNTIME_HEADER \"syntaqlite_runtime.h\"\n");
        source.push_str("#endif\n");
        source.push_str("#include SYNTAQLITE_RUNTIME_HEADER\n\n");
        source.push_str(&format!("#include \"syntaqlite_{d}.h\"\n\n"));
    } else {
        let header_name = if guard == "SYNTAQLITE_RUNTIME_H" {
            "syntaqlite_runtime.h".to_string()
        } else {
            let stem = guard
                .strip_prefix("SYNTAQLITE_")
                .unwrap_or(&guard)
                .strip_suffix("_H")
                .unwrap_or(&guard)
                .to_lowercase();
            format!("syntaqlite_{stem}.h")
        };
        source.push_str(&format!("#include \"{header_name}\"\n\n"));
    }

    // Handle SYNTAQLITE_INLINE_PARSER_DATA_HEADER: if any file uses it and
    // dialect_parse.h is already inlined, define the macro to nothing.
    let has_inline_macro = files.iter().any(|f| {
        f.content.contains("SYNTAQLITE_INLINE_PARSER_DATA_HEADER")
    });
    if has_inline_macro {
        source.push_str("#define SYNTAQLITE_INLINE_PARSER_DATA_HEADER /* already inlined */\n\n");
    }

    // Internal headers, then sources.
    for &i in &internal_headers {
        emit_file(&files[i], &inlined_keys, &mut source);
    }
    for &i in &sources {
        emit_file(&files[i], &inlined_keys, &mut source);
    }

    Ok(AmalgamateOutput { header, source })
}

/// Emit a single file's content, stripping all `#include "..."` and
/// `#include <...>` directives (already inlined or provided by the runtime
/// amalgamation header) and header guards.
fn emit_file(file: &SourceFile, inlined_keys: &HashSet<&str>, out: &mut String) {
    out.push_str(&format!(
        "/* ======== begin: {} ======== */\n",
        file.include_key
    ));

    let lines: Vec<&str> = file.content.lines().collect();
    let (guard_start, guard_end) = detect_header_guard(&lines);

    for (line_idx, &line) in lines.iter().enumerate() {
        // Skip header guard lines.
        if let Some((gs, ge)) = guard_start.zip(guard_end) {
            if line_idx == gs || line_idx == gs + 1 || line_idx == ge {
                continue;
            }
        }

        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("#include") {
            let rest = rest.trim();
            if rest.starts_with('"') || rest.starts_with('<') {
                // All quoted and system includes are stripped:
                // - Inlined files are already present above in the amalgamation.
                // - Unresolved files are runtime headers provided by the
                //   runtime amalgamation or the self-include at the top.
                // - System includes are emitted once at the top.
                continue;
            }
            // Macro includes (e.g. `#include SYNTAQLITE_INLINE_PARSER_DATA_HEADER`)
            // are kept as-is.
        }

        out.push_str(line);
        out.push('\n');
    }

    out.push_str(&format!(
        "/* ======== end: {} ======== */\n\n",
        file.include_key
    ));
}

/// Detect `#ifndef GUARD` / `#define GUARD` at the start and `#endif` at the end.
fn detect_header_guard(lines: &[&str]) -> (Option<usize>, Option<usize>) {
    let mut first_code = None;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.is_empty()
            || t.starts_with("//")
            || t.starts_with("/*")
            || t.starts_with("**")
            || t.starts_with("*/")
        {
            continue;
        }
        first_code = Some(i);
        break;
    }

    let Some(fc) = first_code else {
        return (None, None);
    };
    let guard_name = match lines[fc].trim().strip_prefix("#ifndef ") {
        Some(name) => name.trim(),
        None => return (None, None),
    };

    if fc + 1 >= lines.len() {
        return (None, None);
    }
    if lines[fc + 1].trim() != format!("#define {guard_name}") {
        return (None, None);
    }

    // Find matching `#endif` at the end.
    for i in (0..lines.len()).rev() {
        let t = lines[i].trim();
        if !t.is_empty() {
            return if t.starts_with("#endif") {
                (Some(fc), Some(i))
            } else {
                (None, None)
            };
        }
    }

    (None, None)
}

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Generates a Rust source file containing the `SQLite` built-in function catalog.
//!
//! Reads `functions.json` (extracted from `SQLite` source) and emits a static
//! array of `FunctionEntry` values with availability rules that can be filtered
//! at runtime by `DialectEnv`.

use std::fmt::Write as _;

use serde::Deserialize;

use super::rust_writer::RustWriter;

// ── JSON schema ─────────────────────────────────────────────────────

#[derive(Deserialize)]
struct FunctionsFile {
    functions: Vec<JsonFunction>,
}

#[derive(Deserialize)]
struct JsonFunction {
    name: String,
    arities: Vec<i16>,
    category: String,
    availability: Vec<JsonAvailability>,
}

#[derive(Deserialize)]
struct JsonAvailability {
    since: String,
    until: Option<String>,
    cflag: Option<String>,
    polarity: Option<String>,
}

// ── Version encoding ────────────────────────────────────────────────

/// Convert a version string like `"3.38.5"` to a `SqliteVersion` variant
/// name like `"SqliteVersion::V3_38"`. The patch component is ignored since
/// `SqliteVersion` only tracks major.minor.
fn encode_version(s: &str) -> Result<String, String> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() < 2 {
        return Err(format!(
            "bad version string '{s}': expected at least major.minor"
        ));
    }
    let major: u32 = parts[0]
        .parse()
        .map_err(|e| format!("bad major in version '{s}': {e}"))?;
    let minor: u32 = parts[1]
        .parse()
        .map_err(|e| format!("bad minor in version '{s}': {e}"))?;
    if major != 3 {
        return Err(format!(
            "unsupported major version {major} in '{s}': only SQLite v3 is supported"
        ));
    }
    Ok(format!("SqliteVersion::V3_{minor}"))
}

// ── Cflag name → index mapping ──────────────────────────────────────

/// Map a cflag name (e.g. `"SQLITE_OMIT_JSON"`) to its `SYNQ_CFLAG_IDX_*` index.
///
/// The mapping is derived from the C header constants. We hardcode the known
/// set here rather than parsing the header at codegen time, since the set is
/// stable and tracked by the cflags header.
pub(crate) fn cflag_index(name: &str) -> Option<u32> {
    let suffix = name.strip_prefix("SQLITE_")?;
    match suffix {
        // Parser flags (0–21): indices match C compact SYNQ_CFLAG_IDX_* values.
        "OMIT_ALTERTABLE" => Some(0),
        "OMIT_ANALYZE" => Some(1),
        "OMIT_ATTACH" => Some(2),
        "OMIT_AUTOINCREMENT" => Some(3),
        "OMIT_CAST" => Some(4),
        "OMIT_COMPOUND_SELECT" => Some(5),
        "OMIT_CTE" => Some(6),
        "OMIT_EXPLAIN" => Some(7),
        "OMIT_FOREIGN_KEY" => Some(8),
        "OMIT_GENERATED_COLUMNS" => Some(9),
        "OMIT_PRAGMA" => Some(10),
        "OMIT_REINDEX" => Some(11),
        "OMIT_RETURNING" => Some(12),
        "OMIT_SUBQUERY" => Some(13),
        "OMIT_TEMPDB" => Some(14),
        "OMIT_TRIGGER" => Some(15),
        "OMIT_VACUUM" => Some(16),
        "OMIT_VIEW" => Some(17),
        "OMIT_VIRTUALTABLE" => Some(18),
        "OMIT_WINDOWFUNC" => Some(19),
        "ENABLE_ORDERED_SET_AGGREGATES" => Some(20),
        "ENABLE_UPDATE_DELETE_LIMIT" => Some(21),
        // Non-parser flags (22–41).
        "OMIT_COMPILEOPTION_DIAGS" => Some(22),
        "OMIT_DATETIME_FUNCS" => Some(23),
        "OMIT_FLOATING_POINT" => Some(24),
        "OMIT_JSON" => Some(25),
        "OMIT_LOAD_EXTENSION" => Some(26),
        "ENABLE_BYTECODE_VTAB" => Some(27),
        "ENABLE_CARRAY" => Some(28),
        "ENABLE_DBPAGE_VTAB" => Some(29),
        "ENABLE_DBSTAT_VTAB" => Some(30),
        "ENABLE_FTS3" => Some(31),
        "ENABLE_FTS4" => Some(32),
        "ENABLE_FTS5" => Some(33),
        "ENABLE_GEOPOLY" => Some(34),
        "ENABLE_JSON1" => Some(35),
        "ENABLE_MATH_FUNCTIONS" => Some(36),
        "ENABLE_OFFSET_SQL_FUNC" => Some(37),
        "ENABLE_PERCENTILE" => Some(38),
        "ENABLE_RTREE" => Some(39),
        "ENABLE_STMTVTAB" => Some(40),
        "SOUNDEX" => Some(41),
        _ => None,
    }
}

// ── Code generation ─────────────────────────────────────────────────

/// Read `functions.json` from `json_path`, generate the catalog Rust source, and
/// write it to `output_path` (creating parent directories as needed).
///
/// # Errors
///
/// Returns an error if reading, parsing, or writing fails.
pub(crate) fn write_functions_catalog_file(
    json_path: &str,
    output_path: &str,
) -> Result<(), String> {
    use std::fs;
    use std::path::Path;

    let json = fs::read_to_string(json_path).map_err(|e| format!("reading {json_path}: {e}"))?;
    let content = generate_functions_catalog(&json)?;
    let out = Path::new(output_path);
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("creating output directory: {e}"))?;
    }
    fs::write(out, content).map_err(|e| format!("writing {}: {e}", out.display()))?;
    eprintln!("wrote {output_path}");
    Ok(())
}

/// Generate the `functions_catalog.rs` Rust source from `functions.json` content.
///
/// # Errors
///
/// Returns an error if JSON parsing fails or an unknown category/cflag is encountered.
#[expect(clippy::too_many_lines)]
pub(crate) fn generate_functions_catalog(json_content: &str) -> Result<String, String> {
    let file: FunctionsFile =
        serde_json::from_str(json_content).map_err(|e| format!("parsing functions.json: {e}"))?;

    let mut w = RustWriter::new();
    w.file_header();
    w.line("//! Static catalog of `SQLite` built-in functions with version/cflag availability.");
    w.newline();
    w.line("use crate::dialect::{AvailabilityRule, CflagPolarity, FunctionCategory, FunctionEntry, FunctionInfo, SqliteVersion};");
    w.newline();

    // Static data: arity arrays
    for func in &file.functions {
        let ident = arity_ident(&func.name);
        let arities = func
            .arities
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(w, "static ARITIES_{ident}: &[i16] = &[{arities}];");
    }
    w.newline();

    // Static data: availability arrays
    for func in &file.functions {
        let ident = arity_ident(&func.name);
        let mut entries: Vec<String> = Vec::new();
        for avail in &func.availability {
            let since = encode_version(&avail.since)?;
            let until = match &avail.until {
                Some(v) => format!("Some({})", encode_version(v)?),
                None => "None".to_string(),
            };
            let (cflag_idx_str, polarity) = match &avail.cflag {
                Some(name) => {
                    let idx = cflag_index(name).ok_or_else(|| {
                        format!("unknown cflag '{}' in function '{}'", name, func.name)
                    })?;
                    let pol = match avail.polarity.as_deref() {
                        Some("enable") => "CflagPolarity::Enable",
                        Some("omit") => "CflagPolarity::Omit",
                        other => {
                            return Err(format!(
                                "unknown polarity '{:?}' for cflag '{}' in function '{}'",
                                other, name, func.name
                            ));
                        }
                    };
                    (format!("{idx}"), pol)
                }
                None => ("u32::MAX".to_string(), "CflagPolarity::Enable"),
            };
            entries.push(format!(
                "AvailabilityRule {{ since: {since}, until: {until}, cflag_index: {cflag_idx_str}, cflag_polarity: {polarity} }}"
            ));
        }
        let entries_str = entries.join(", ");
        let _ = writeln!(
            w,
            "static AVAIL_{ident}: &[AvailabilityRule] = &[{entries_str}];"
        );
    }
    w.newline();

    // Main catalog array
    let count = file.functions.len();
    let _ = writeln!(w, "/// All {count} `SQLite` built-in functions.");
    w.line("pub(crate) static SQLITE_FUNCTIONS: &[FunctionEntry<'static>] = &[");
    w.indent();
    for func in &file.functions {
        let ident = arity_ident(&func.name);
        let cat = match func.category.as_str() {
            "scalar" => "FunctionCategory::Scalar",
            "aggregate" => "FunctionCategory::Aggregate",
            "window" => "FunctionCategory::Window",
            "table_valued" => "FunctionCategory::TableValued",
            other => {
                return Err(format!(
                    "unknown category '{other}' for function '{}'",
                    func.name
                ));
            }
        };
        let name_escaped = func.name.replace('\\', "\\\\").replace('"', "\\\"");
        w.open_block("FunctionEntry {");
        let _ = writeln!(
            w,
            "info: FunctionInfo {{ name: \"{name_escaped}\", arities: ARITIES_{ident}, category: {cat} }},"
        );
        let _ = writeln!(w, "availability: AVAIL_{ident},");
        w.close_block("},");
    }
    w.close_block("];");
    w.newline();

    Ok(w.finish())
}

/// Convert a function name to a valid Rust identifier for use in static names.
fn arity_ident(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => out.push(ch.to_ascii_uppercase()),
            '-' | '>' => out.push('_'),
            _ => {
                // Fallback: hex encode
                let _ = write!(out, "X{:02X}", ch as u32);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arity_ident_basic() {
        assert_eq!(arity_ident("abs"), "ABS");
        assert_eq!(arity_ident("->"), "__");
        assert_eq!(arity_ident("->>"), "___");
        assert_eq!(arity_ident("json_array"), "JSON_ARRAY");
    }

    #[test]
    fn generate_from_minimal_json() {
        let json = r#"{
            "functions": [
                {
                    "name": "abs",
                    "arities": [0, 1],
                    "category": "scalar",
                    "availability": [
                        { "since": "3.30.1" }
                    ]
                }
            ]
        }"#;
        let result = generate_functions_catalog(json).unwrap();
        assert!(result.contains("SQLITE_FUNCTIONS"));
        assert!(result.contains("\"abs\""));
        assert!(result.contains("FunctionCategory::Scalar"));
        assert!(result.contains("use crate::dialect::"));
    }

    #[test]
    fn generate_with_cflag() {
        let json = r#"{
            "functions": [
                {
                    "name": "acos",
                    "arities": [1],
                    "category": "scalar",
                    "availability": [
                        {
                            "since": "3.35.5",
                            "cflag": "SQLITE_ENABLE_MATH_FUNCTIONS",
                            "polarity": "enable"
                        }
                    ]
                }
            ]
        }"#;
        let result = generate_functions_catalog(json).unwrap();
        assert!(result.contains("cflag_index: 36"));
        assert!(result.contains("CflagPolarity::Enable"));
    }
}

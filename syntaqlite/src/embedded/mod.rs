// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Embedded SQL extraction from host language sources.
//!
//! Extracts SQL fragments from host language files, replaces interpolation holes
//! with placeholder identifiers, parses the SQL with `begin_macro`/`end_macro`
//! wrapping each hole, runs validation, and maps diagnostic offsets back to
//! host-file positions.
//!
//! Language-specific extractors live in submodules:
//! - [`extract_python`] — Python f-string extraction
//! - [`extract_typescript`] — TypeScript/JavaScript template literal extraction

pub mod offset_map;
mod python;
mod typescript;

pub use python::extract_python;
pub use typescript::extract_typescript;

use std::ops::Range;

use crate::dialect::Dialect;
use crate::dialect::TokenCategory;
use crate::parser::incremental::RawIncrementalParser;
use crate::parser::session::ParseError;
use crate::parser::tokenizer::RawTokenizer;
use crate::validation::{Diagnostic, DiagnosticMessage, FunctionDef, ValidationConfig};

use offset_map::OffsetMap;

// ── Shared types ────────────────────────────────────────────────────────

/// A SQL fragment extracted from a host language source file.
#[derive(Debug)]
pub struct EmbeddedFragment {
    /// Byte range of the SQL content in the host file (excluding quotes).
    pub sql_range: Range<usize>,
    /// SQL text with holes replaced by placeholder identifiers.
    pub sql_text: String,
    /// Information about each interpolation hole.
    pub holes: Vec<Hole>,
}

/// An interpolation hole (e.g. `{expr}` in a Python f-string, `${expr}` in JS).
#[derive(Debug)]
pub struct Hole {
    /// Byte range of the hole expression in the host file.
    pub host_range: Range<usize>,
    /// Byte offset in `sql_text` where the placeholder sits.
    pub sql_offset: usize,
    /// The placeholder identifier (e.g. `__hole_0__`).
    pub placeholder: String,
}

// ── Shared scanner utilities ────────────────────────────────────────────

/// SQL keywords that identify a string as containing SQL.
const SQL_KEYWORDS: &[&str] = &[
    "SELECT",
    "INSERT",
    "UPDATE",
    "DELETE",
    "CREATE",
    "ALTER",
    "DROP",
    "WITH",
    "EXPLAIN",
    "PRAGMA",
    "ATTACH",
    "DETACH",
    "REINDEX",
    "VACUUM",
    "BEGIN",
    "COMMIT",
    "ROLLBACK",
    "SAVEPOINT",
    "RELEASE",
];

/// Check if the given text starts with a SQL keyword (case-insensitive).
fn starts_with_sql_keyword(text: &str) -> bool {
    let trimmed = text.trim_start();
    for kw in SQL_KEYWORDS {
        if trimmed.len() >= kw.len()
            && trimmed[..kw.len()].eq_ignore_ascii_case(kw)
            && (trimmed.len() == kw.len() || !trimmed.as_bytes()[kw.len()].is_ascii_alphanumeric())
        {
            return true;
        }
    }
    false
}

/// Skip a single-line string literal (`"..."` or `'...'`) with backslash escapes.
///
/// Shared by both Python (non-triple-quote case) and TypeScript/JavaScript
/// extractors. Terminates at the matching quote, a newline, or end of input.
fn skip_single_line_string(bytes: &[u8], pos: usize, end: usize) -> usize {
    let quote = bytes[pos];
    let mut j = pos + 1;
    while j < end {
        if bytes[j] == b'\\' {
            j += 2;
            continue;
        }
        if bytes[j] == quote {
            return j + 1;
        }
        if bytes[j] == b'\n' {
            return j;
        }
        j += 1;
    }
    j
}

// ── Hole-aware parsing and validation ───────────────────────────────────

/// Validate all SQL fragments extracted from a host source file.
///
/// `functions` is the list of known built-in and user-defined functions.
/// Pass `&[]` to skip function validation, or use
/// [`sqlite_function_defs`] to get the SQLite built-in catalog.
///
/// Returns diagnostics with offsets mapped back to host-file positions.
pub fn validate_embedded(
    dialect: &Dialect,
    fragments: &[EmbeddedFragment],
    functions: &[FunctionDef],
    config: &ValidationConfig,
) -> Vec<Diagnostic> {
    let mut all_diags = Vec::new();

    for fragment in fragments {
        let diags = validate_fragment(dialect, fragment, functions, config);
        let offset_map = OffsetMap::new(fragment);

        for mut d in diags {
            if is_hole_diagnostic(&d, fragment) {
                continue;
            }
            // Map offsets back to host positions; skip diagnostics that
            // fall entirely inside a hole placeholder.
            let Some(start) = offset_map.to_host(d.start_offset) else {
                continue;
            };
            let end = offset_map.to_host(d.end_offset).unwrap_or(start);
            d.start_offset = start;
            d.end_offset = end;
            all_diags.push(d);
        }
    }

    all_diags
}

/// Build `FunctionDef` entries from the SQLite built-in function catalog.
///
/// Uses default `DialectConfig` (latest version, no cflags) to determine
/// which functions are available.
#[cfg(feature = "sqlite")]
pub fn sqlite_function_defs() -> Vec<FunctionDef> {
    let config = syntaqlite_parser::dialect::ffi::DialectConfig::default();
    syntaqlite_parser::sqlite::available_functions(&config)
        .into_iter()
        .flat_map(|info| crate::validation::expand_function_info(info))
        .collect()
}

/// Compute semantic tokens for a single embedded SQL fragment.
///
/// Uses the full parser (with `collect_tokens`) to get accurate flag-based
/// token classification (e.g. `datetime` → Function when used as a callee).
///
/// Returns `(sql_offset, length, category)` tuples with byte offsets into
/// `fragment.sql_text`. The caller is responsible for mapping these through
/// an [`OffsetMap`] to host-file positions.
pub fn fragment_semantic_tokens(
    dialect: &Dialect,
    fragment: &EmbeddedFragment,
) -> Vec<(usize, usize, TokenCategory)> {
    let mut parser = RawIncrementalParser::builder(dialect).build();
    let mut tokenizer = RawTokenizer::builder(*dialect).build();

    // Tokenize the processed SQL text.
    let tokens: Vec<(u32, usize, usize)> = {
        let cursor = tokenizer.tokenize(&fragment.sql_text);
        cursor
            .map(|tok| {
                let offset = tok.text.as_ptr() as usize - fragment.sql_text.as_ptr() as usize;
                (tok.token_type, offset, tok.text.len())
            })
            .collect()
    };

    // Feed tokens to the parser with hole wrapping (same loop as validate_fragment).
    let mut cursor = parser.feed(&fragment.sql_text);

    for &(token_type, offset, length) in &tokens {
        let hole = fragment
            .holes
            .iter()
            .find(|h| offset >= h.sql_offset && offset < h.sql_offset + h.placeholder.len());

        if let Some(hole) = hole {
            cursor.begin_macro(hole.host_range.start as u32, hole.host_range.len() as u32);
        }

        // Ignore parse results — we only need the collected tokens.
        let _ = cursor.feed_token(token_type, offset..offset + length);

        if hole.is_some() {
            cursor.end_macro();
        }
    }

    let _ = cursor.finish();

    let mut result = Vec::new();

    // Classify non-whitespace, non-comment tokens using parser flags.
    for tp in cursor.state().tokens() {
        let cat = TokenCategory::from_u8(dialect.classify_token_raw(tp.type_, tp.flags));
        if cat == TokenCategory::Other {
            continue;
        }
        result.push((tp.offset as usize, tp.length as usize, cat));
    }

    // Add comments as Comment tokens.
    for c in cursor.state().comments() {
        result.push((c.offset as usize, c.length as usize, TokenCategory::Comment));
    }

    result
}

/// Tokenize, parse (with hole wrapping), and validate a single fragment.
///
/// Returns diagnostics with SQL-text byte offsets (not yet mapped to host).
fn validate_fragment(
    dialect: &Dialect,
    fragment: &EmbeddedFragment,
    functions: &[FunctionDef],
    config: &ValidationConfig,
) -> Vec<Diagnostic> {
    let mut parser = RawIncrementalParser::builder(dialect).build();
    let mut tokenizer = RawTokenizer::builder(*dialect).build();

    // Tokenize the processed SQL text.
    let tokens: Vec<(u32, usize, usize)> = {
        let cursor = tokenizer.tokenize(&fragment.sql_text);
        cursor
            .map(|tok| {
                let offset = tok.text.as_ptr() as usize - fragment.sql_text.as_ptr() as usize;
                (tok.token_type, offset, tok.text.len())
            })
            .collect()
    };

    // Feed tokens to the low-level parser, collecting results.
    let mut cursor = parser.feed(&fragment.sql_text);
    let mut results: Vec<Result<crate::parser::nodes::NodeId, ParseError>> = Vec::new();

    for &(token_type, offset, length) in &tokens {
        let hole = fragment
            .holes
            .iter()
            .find(|h| offset >= h.sql_offset && offset < h.sql_offset + h.placeholder.len());

        if let Some(hole) = hole {
            cursor.begin_macro(hole.host_range.start as u32, hole.host_range.len() as u32);
        }

        match cursor.feed_token(token_type, offset..offset + length) {
            Ok(Some(root)) => results.push(Ok(root)),
            Ok(None) => {}
            Err(e) => results.push(Err(e)),
        }

        if hole.is_some() {
            cursor.end_macro();
        }
    }

    match cursor.finish() {
        Ok(Some(root)) => results.push(Ok(root)),
        Ok(None) => {}
        Err(e) => results.push(Err(e)),
    }

    // After finish(), the cursor's reader still points at valid arena data.
    crate::validation::validate_parse_results(
        cursor.state().reader(),
        &results,
        &fragment.sql_text,
        dialect,
        None,
        functions,
        config,
    )
}

/// Check if a diagnostic message references a hole placeholder name.
fn is_hole_diagnostic(diag: &Diagnostic, fragment: &EmbeddedFragment) -> bool {
    match &diag.message {
        DiagnosticMessage::UnknownTable { name } | DiagnosticMessage::UnknownFunction { name } => {
            fragment.holes.iter().any(|h| h.placeholder == *name)
        }
        DiagnosticMessage::UnknownColumn { column, .. } => {
            fragment.holes.iter().any(|h| h.placeholder == *column)
        }
        _ => false,
    }
}

/// Produce LSP-encoded semantic tokens for a host-language source containing embedded SQL.
///
/// Gathers per-fragment semantic tokens via [`fragment_semantic_tokens`], maps each token
/// to its host-file byte offset via [`OffsetMap`], and delta-encodes the result into
/// the `[deltaLine, deltaStart, length, tokenType, modifiers]` 5-tuple format consumed
/// by LSP `textDocument/semanticTokens` responses.
///
/// This is the embedded analogue of `AnalysisHost::semantic_tokens_encoded`.
pub fn embedded_semantic_tokens_encoded(
    dialect: &Dialect,
    fragments: &[EmbeddedFragment],
    source: &str,
) -> Vec<u32> {
    let source_bytes = source.as_bytes();

    // Collect (host_offset, length, legend_idx) for all fragments.
    let mut all_tokens: Vec<(usize, usize, u32)> = Vec::new();
    for fragment in fragments {
        let offset_map = OffsetMap::new(fragment);
        for (sql_offset, length, cat) in fragment_semantic_tokens(dialect, fragment) {
            let Some(legend_idx) = cat.legend_index() else {
                continue;
            };
            let Some(host_offset) = offset_map.to_host(sql_offset) else {
                // Inside a hole placeholder — not real SQL text.
                continue;
            };
            let host_len = length.min(source.len().saturating_sub(host_offset));
            if host_len == 0 {
                continue;
            }
            all_tokens.push((host_offset, host_len, legend_idx));
        }
    }

    // Sort by host offset before delta-encoding.
    all_tokens.sort_by_key(|t| t.0);

    // Delta-encode: advance a line/col cursor through the source and emit
    // deltas relative to the previous token's start position.
    let mut result: Vec<u32> = Vec::with_capacity(all_tokens.len() * 5);
    let mut prev_line: u32 = 0;
    let mut prev_col: u32 = 0;
    let mut cur_line: u32 = 0;
    let mut cur_col: u32 = 0;
    let mut src_pos: usize = 0;

    for (host_offset, host_len, legend_idx) in all_tokens {
        while src_pos < host_offset && src_pos < source_bytes.len() {
            if source_bytes[src_pos] == b'\n' {
                cur_line += 1;
                cur_col = 0;
            } else {
                cur_col += 1;
            }
            src_pos += 1;
        }
        let delta_line = cur_line - prev_line;
        let delta_start = if delta_line == 0 {
            cur_col - prev_col
        } else {
            cur_col
        };
        result.push(delta_line);
        result.push(delta_start);
        result.push(host_len as u32);
        result.push(legend_idx);
        result.push(0); // modifiers
        prev_line = cur_line;
        prev_col = cur_col;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::Severity;

    fn dialect() -> Dialect<'static> {
        *crate::sqlite::DIALECT
    }

    fn default_config() -> ValidationConfig {
        ValidationConfig::default()
    }

    fn builtin_functions() -> Vec<FunctionDef> {
        sqlite_function_defs()
    }

    /// Helper: extract Python fragments then validate, returning only parse errors.
    fn parse_errors_python(source: &str) -> Vec<Diagnostic> {
        let d = dialect();
        let fragments = extract_python(source);
        let all = validate_embedded(&d, &fragments, &builtin_functions(), &default_config());
        all.into_iter()
            .filter(|d| d.message.is_parse_error())
            .collect()
    }

    /// Helper: extract TypeScript fragments then validate, returning only parse errors.
    fn parse_errors_typescript(source: &str) -> Vec<Diagnostic> {
        let d = dialect();
        let fragments = extract_typescript(source);
        let all = validate_embedded(&d, &fragments, &builtin_functions(), &default_config());
        all.into_iter()
            .filter(|d| d.message.is_parse_error())
            .collect()
    }

    // ── Python syntax error tests ────────────────────────────────────

    #[test]
    fn python_valid_sql_no_errors() {
        let source = r#"db.execute(f"SELECT id, name FROM users WHERE id = {uid}")"#;
        let diags = parse_errors_python(source);
        assert!(diags.is_empty(), "expected no parse errors, got: {diags:?}");
    }

    #[test]
    fn python_syntax_error_missing_expr_list() {
        // "SELECT FROM t" is a syntax error — missing expression list.
        let source = r#"db.execute(f"SELECT FROM t")"#;
        let diags = parse_errors_python(source);
        assert!(!diags.is_empty(), "expected parse error for 'SELECT FROM'");
        assert!(diags.iter().all(|d| d.severity == Severity::Error));
    }

    #[test]
    fn python_syntax_error_misspelled_from() {
        // "SELECT * FORM t" — FORM is not a keyword after *.
        let source = r#"db.execute(f"SELECT * FORM t")"#;
        let diags = parse_errors_python(source);
        assert!(!diags.is_empty(), "expected parse error for 'FORM'");
    }

    #[test]
    fn python_syntax_error_double_where() {
        let source = r#"db.execute(f"SELECT id FROM t WHERE x = 1 WHERE y = 2")"#;
        let diags = parse_errors_python(source);
        assert!(!diags.is_empty(), "expected parse error for double WHERE");
    }

    #[test]
    fn python_syntax_error_offset_in_host() {
        // The error offset should be mapped to the host file position,
        // not the raw SQL fragment position.
        let source = r#"prefix = 1; db.execute(f"SELECT FROM t")"#;
        let fragments = extract_python(source);
        assert_eq!(fragments.len(), 1);
        let d = dialect();
        let diags = validate_embedded(&d, &fragments, &builtin_functions(), &default_config());
        let parse_diags: Vec<_> = diags
            .into_iter()
            .filter(|d| d.message.is_parse_error())
            .collect();
        assert!(!parse_diags.is_empty(), "expected parse error");
        // The diagnostic offset should be within the f-string region, not at 0.
        let fstring_start = source.find("SELECT").unwrap();
        assert!(
            parse_diags[0].start_offset >= fstring_start,
            "expected offset >= {fstring_start}, got {}",
            parse_diags[0].start_offset,
        );
    }

    #[test]
    fn python_multiple_fragments_only_second_errors() {
        let source = concat!("a = f\"SELECT id FROM t\"\n", "b = f\"SELECT FROM t\"\n",);
        let diags = parse_errors_python(source);
        assert!(!diags.is_empty(), "expected parse error in second fragment");
        // First fragment is valid, so all errors should be in the second f-string.
        let second_select = source.rfind("SELECT").unwrap();
        for d in &diags {
            assert!(
                d.start_offset >= second_select,
                "error at offset {} is before second fragment start {second_select}",
                d.start_offset,
            );
        }
    }

    #[test]
    fn python_valid_with_hole_no_errors() {
        let source = r#"db.execute(f"INSERT INTO t (a, b) VALUES ({x}, {y})")"#;
        let diags = parse_errors_python(source);
        assert!(diags.is_empty(), "expected no parse errors, got: {diags:?}");
    }

    // ── TypeScript syntax error tests ────────────────────────────────

    #[test]
    fn typescript_valid_sql_no_errors() {
        let source = "db.prepare(`SELECT id, name FROM users WHERE id = ${uid}`).all();";
        let diags = parse_errors_typescript(source);
        assert!(diags.is_empty(), "expected no parse errors, got: {diags:?}");
    }

    #[test]
    fn typescript_syntax_error_missing_expr_list() {
        let source = "db.prepare(`SELECT FROM users`).all();";
        let diags = parse_errors_typescript(source);
        assert!(!diags.is_empty(), "expected parse error for 'SELECT FROM'");
        assert!(diags.iter().all(|d| d.severity == Severity::Error));
    }

    #[test]
    fn typescript_syntax_error_double_where() {
        let source = "db.prepare(`SELECT id FROM t WHERE x = 1 WHERE y = 2`).all();";
        let diags = parse_errors_typescript(source);
        assert!(!diags.is_empty(), "expected parse error for double WHERE");
    }

    // ── Semantic diagnostics are included but separable ──────────────

    #[test]
    fn semantic_diagnostics_present_for_unknown_table() {
        let source = r#"db.execute(f"SELECT id FROM unknown_tbl")"#;
        let d = dialect();
        let fragments = extract_python(source);
        let all = validate_embedded(&d, &fragments, &builtin_functions(), &default_config());
        // Should have semantic diagnostics (unknown table) but no parse errors.
        let parse: Vec<_> = all.iter().filter(|d| d.message.is_parse_error()).collect();
        let semantic: Vec<_> = all.iter().filter(|d| !d.message.is_parse_error()).collect();
        assert!(parse.is_empty(), "no parse errors expected");
        assert!(
            !semantic.is_empty(),
            "expected semantic diagnostic for unknown table"
        );
    }

    #[test]
    fn python_syntax_error_offset_points_to_typo() {
        // "VALUS" is a typo for "VALUES" — the error span should point to
        // VALUS, not to INSERT (the start of the statement).
        let source = r#"conn.execute(f"INSERT INTO orders (a, b) VALUS ({x}, {y})")"#;
        let fragments = extract_python(source);
        assert_eq!(fragments.len(), 1);
        let d = dialect();
        let diags = validate_embedded(&d, &fragments, &builtin_functions(), &default_config());
        let parse_diags: Vec<_> = diags
            .into_iter()
            .filter(|d| d.message.is_parse_error())
            .collect();
        assert!(!parse_diags.is_empty(), "expected parse error for VALUS");
        let valus_start = source.find("VALUS").unwrap();
        let valus_end = valus_start + "VALUS".len();
        assert_eq!(
            parse_diags[0].start_offset, valus_start,
            "error start should point to VALUS (offset {valus_start}), got {}",
            parse_diags[0].start_offset,
        );
        assert_eq!(
            parse_diags[0].end_offset, valus_end,
            "error end should span VALUS (offset {valus_end}), got {}",
            parse_diags[0].end_offset,
        );
    }

    #[test]
    fn python_builtin_function_not_flagged() {
        // datetime() is a built-in SQLite function — should not produce
        // an "unknown function" diagnostic.
        let source = r#"db.execute(f"INSERT INTO t (a) VALUES (datetime('now'))")"#;
        let d = dialect();
        let fragments = extract_python(source);
        let all = validate_embedded(&d, &fragments, &builtin_functions(), &default_config());
        let unknown_fn: Vec<_> = all
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownFunction { .. }))
            .collect();
        assert!(
            unknown_fn.is_empty(),
            "datetime should not be flagged as unknown, got: {unknown_fn:?}",
        );
    }

    #[test]
    fn semantic_tokens_classify_function_callee() {
        // datetime('now') should classify `datetime` as Function, not Identifier.
        // This was broken when the embedded path used a raw Tokenizer (no parser flags).
        let source = r#"db.execute(f"INSERT INTO t (a) VALUES (datetime('now'))")"#;
        let d = dialect();
        let fragments = extract_python(source);
        assert_eq!(fragments.len(), 1);
        let tokens = fragment_semantic_tokens(&d, &fragments[0]);
        let datetime_tokens: Vec<_> = tokens
            .iter()
            .filter(|(off, len, _)| &fragments[0].sql_text[*off..*off + *len] == "datetime")
            .collect();
        assert_eq!(
            datetime_tokens.len(),
            1,
            "expected exactly one 'datetime' token, got: {datetime_tokens:?}",
        );
        assert_eq!(
            datetime_tokens[0].2,
            TokenCategory::Function,
            "datetime should be classified as Function, got {:?}",
            datetime_tokens[0].2,
        );
    }

    #[test]
    fn hole_diagnostics_filtered_out() {
        // Holes should not produce unknown-table/column diagnostics.
        let source = r#"db.execute(f"SELECT {col} FROM {tbl}")"#;
        let d = dialect();
        let fragments = extract_python(source);
        let all = validate_embedded(&d, &fragments, &builtin_functions(), &default_config());
        // Hole placeholders should be filtered — no diagnostics about __hole_N__.
        for diag in &all {
            let msg = format!("{}", diag.message);
            assert!(
                !msg.contains("__hole_"),
                "hole placeholder leaked into diagnostics: {msg}",
            );
        }
    }
}

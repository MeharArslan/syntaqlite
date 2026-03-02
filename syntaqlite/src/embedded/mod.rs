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

use crate::dialect::{DialectExt, TokenCategory};
use crate::semantic::functions::FunctionCatalog;
use crate::validation::ValidationConfig;
use crate::validation::types::{Diagnostic, DiagnosticMessage};
use syntaqlite_parser::ParseError;
use syntaqlite_parser::RawDialect;
use syntaqlite_parser::RawIncrementalParser;
use syntaqlite_parser::RawTokenizer;

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

// ── EmbeddedAnalyzer ────────────────────────────────────────────────────

/// Analyzer for embedded SQL in host-language source files.
///
/// Holds the dialect, optional function catalog, and validation config so they
/// don't need to be threaded through every call.
///
/// # Example
///
/// ```rust,no_run
/// # use syntaqlite::embedded::{EmbeddedAnalyzer, extract_python};
/// # use syntaqlite::semantic::functions::FunctionCatalog;
/// # let source = "";
/// # let dialect = syntaqlite::dialect::sqlite();
/// let catalog = FunctionCatalog::for_default_dialect(&dialect);
/// let fragments = extract_python(source);
/// let diags = EmbeddedAnalyzer::new(dialect)
///     .with_catalog(catalog)
///     .validate(&fragments);
/// ```
pub struct EmbeddedAnalyzer<'d> {
    dialect: RawDialect<'d>,
    catalog: FunctionCatalog,
    config: ValidationConfig,
}

impl<'d> EmbeddedAnalyzer<'d> {
    /// Create a new analyzer with an empty function catalog and default
    /// validation config.
    pub fn new(dialect: RawDialect<'d>) -> Self {
        Self {
            dialect,
            catalog: FunctionCatalog::for_dialect(
                &dialect,
                &syntaqlite_parser::DialectConfig::default(),
            ),
            config: ValidationConfig::default(),
        }
    }

    /// Attach a function catalog to enable function-existence validation.
    pub fn with_catalog(mut self, catalog: FunctionCatalog) -> Self {
        self.catalog = catalog;
        self
    }

    /// Override the default validation config.
    pub fn with_config(mut self, config: ValidationConfig) -> Self {
        self.config = config;
        self
    }

    /// Validate all SQL fragments and return diagnostics mapped to host-file positions.
    ///
    /// Diagnostics whose spans fall entirely inside a hole placeholder are
    /// filtered out.
    pub fn validate(&self, fragments: &[EmbeddedFragment]) -> Vec<Diagnostic> {
        let mut all_diags = Vec::new();

        for fragment in fragments {
            let diags = self.validate_fragment(fragment);
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

    /// Compute semantic tokens for a single embedded SQL fragment.
    ///
    /// Uses the full parser (with `collect_tokens`) to get accurate flag-based
    /// token classification (e.g. `datetime` → Function when used as a callee).
    ///
    /// Returns `(sql_offset, length, category)` tuples with byte offsets into
    /// `fragment.sql_text`. The caller is responsible for mapping these through
    /// an [`OffsetMap`] to host-file positions.
    pub fn fragment_semantic_tokens(
        &self,
        fragment: &EmbeddedFragment,
    ) -> Vec<(usize, usize, TokenCategory)> {
        let dialect = self.dialect;
        let mut parser = RawIncrementalParser::new(dialect);
        let mut tokenizer = RawTokenizer::new(dialect);

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

        // Feed tokens to the parser with hole wrapping.
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
        for tp in cursor.tokens() {
            let cat = dialect.classify_token(tp.type_, tp.flags);
            if cat == TokenCategory::Other {
                continue;
            }
            result.push((tp.offset as usize, tp.length as usize, cat));
        }

        // Add comments as Comment tokens.
        for c in cursor.comments() {
            result.push((c.offset as usize, c.length as usize, TokenCategory::Comment));
        }

        result
    }

    /// Produce LSP-encoded semantic tokens for a host-language source containing
    /// embedded SQL.
    ///
    /// Gathers per-fragment semantic tokens, maps each token to its host-file
    /// byte offset via [`OffsetMap`], and delta-encodes the result into the
    /// `[deltaLine, deltaStart, length, tokenType, modifiers]` 5-tuple format
    /// consumed by LSP `textDocument/semanticTokens` responses.
    pub fn semantic_tokens_encoded(
        &self,
        fragments: &[EmbeddedFragment],
        source: &str,
    ) -> Vec<u32> {
        let source_bytes = source.as_bytes();

        // Collect (host_offset, length, legend_idx) for all fragments.
        let mut all_tokens: Vec<(usize, usize, u32)> = Vec::new();
        for fragment in fragments {
            let offset_map = OffsetMap::new(fragment);
            for (sql_offset, length, cat) in self.fragment_semantic_tokens(fragment) {
                if cat == TokenCategory::Other {
                    continue;
                }
                let legend_idx = cat as u32;
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

        // Delta-encode into LSP 5-tuple format.
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

    /// Tokenize, parse (with hole wrapping), and validate a single fragment.
    ///
    /// Returns diagnostics with SQL-text byte offsets (not yet mapped to host).
    fn validate_fragment(&self, fragment: &EmbeddedFragment) -> Vec<Diagnostic> {
        let dialect = self.dialect;
        let mut parser = RawIncrementalParser::new(dialect);
        let mut tokenizer = RawTokenizer::new(dialect);

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
        let mut results: Vec<Result<syntaqlite_parser::NodeId, ParseError>> = Vec::new();

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

        crate::validation::validate_parse_results(
            cursor.reader(),
            &results,
            &fragment.sql_text,
            dialect,
            None,
            &self.catalog,
            &self.config,
        )
    }
}

// ── Utilities ────────────────────────────────────────────────────────────

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        embedded::{python::extract_python, typescript::extract_typescript},
        validation::Severity,
    };

    fn analyzer() -> EmbeddedAnalyzer<'static> {
        let dialect = crate::dialect::sqlite();
        let catalog = FunctionCatalog::for_default_dialect(&dialect);
        EmbeddedAnalyzer::new(dialect).with_catalog(catalog)
    }

    // ── Python syntax error tests ────────────────────────────────────

    #[test]
    fn python_valid_sql_no_errors() {
        let source = r#"db.execute(f"SELECT id, name FROM users WHERE id = {uid}")"#;
        let diags = analyzer()
            .validate(&extract_python(source))
            .into_iter()
            .filter(|d| d.message.is_parse_error())
            .collect::<Vec<_>>();
        assert!(diags.is_empty(), "expected no parse errors, got: {diags:?}");
    }

    #[test]
    fn python_syntax_error_missing_expr_list() {
        let source = r#"db.execute(f"SELECT FROM t")"#;
        let diags = analyzer()
            .validate(&extract_python(source))
            .into_iter()
            .filter(|d| d.message.is_parse_error())
            .collect::<Vec<_>>();
        assert!(!diags.is_empty(), "expected parse error for 'SELECT FROM'");
        assert!(diags.iter().all(|d| d.severity == Severity::Error));
    }

    #[test]
    fn python_syntax_error_misspelled_from() {
        let source = r#"db.execute(f"SELECT * FORM t")"#;
        let diags = analyzer()
            .validate(&extract_python(source))
            .into_iter()
            .filter(|d| d.message.is_parse_error())
            .collect::<Vec<_>>();
        assert!(!diags.is_empty(), "expected parse error for 'FORM'");
    }

    #[test]
    fn python_syntax_error_double_where() {
        let source = r#"db.execute(f"SELECT id FROM t WHERE x = 1 WHERE y = 2")"#;
        let diags = analyzer()
            .validate(&extract_python(source))
            .into_iter()
            .filter(|d| d.message.is_parse_error())
            .collect::<Vec<_>>();
        assert!(!diags.is_empty(), "expected parse error for double WHERE");
    }

    #[test]
    fn python_syntax_error_offset_in_host() {
        let source = r#"prefix = 1; db.execute(f"SELECT FROM t")"#;
        let fragments = extract_python(source);
        assert_eq!(fragments.len(), 1);
        let parse_diags: Vec<_> = analyzer()
            .validate(&fragments)
            .into_iter()
            .filter(|d| d.message.is_parse_error())
            .collect();
        assert!(!parse_diags.is_empty(), "expected parse error");
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
        let diags = analyzer()
            .validate(&extract_python(source))
            .into_iter()
            .filter(|d| d.message.is_parse_error())
            .collect::<Vec<_>>();
        assert!(!diags.is_empty(), "expected parse error in second fragment");
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
        let diags = analyzer()
            .validate(&extract_python(source))
            .into_iter()
            .filter(|d| d.message.is_parse_error())
            .collect::<Vec<_>>();
        assert!(diags.is_empty(), "expected no parse errors, got: {diags:?}");
    }

    // ── TypeScript syntax error tests ────────────────────────────────

    #[test]
    fn typescript_valid_sql_no_errors() {
        let source = "db.prepare(`SELECT id, name FROM users WHERE id = ${uid}`).all();";
        let diags = analyzer()
            .validate(&extract_typescript(source))
            .into_iter()
            .filter(|d| d.message.is_parse_error())
            .collect::<Vec<_>>();
        assert!(diags.is_empty(), "expected no parse errors, got: {diags:?}");
    }

    #[test]
    fn typescript_syntax_error_missing_expr_list() {
        let source = "db.prepare(`SELECT FROM users`).all();";
        let diags = analyzer()
            .validate(&extract_typescript(source))
            .into_iter()
            .filter(|d| d.message.is_parse_error())
            .collect::<Vec<_>>();
        assert!(!diags.is_empty(), "expected parse error for 'SELECT FROM'");
        assert!(diags.iter().all(|d| d.severity == Severity::Error));
    }

    #[test]
    fn typescript_syntax_error_double_where() {
        let source = "db.prepare(`SELECT id FROM t WHERE x = 1 WHERE y = 2`).all();";
        let diags = analyzer()
            .validate(&extract_typescript(source))
            .into_iter()
            .filter(|d| d.message.is_parse_error())
            .collect::<Vec<_>>();
        assert!(!diags.is_empty(), "expected parse error for double WHERE");
    }

    // ── Semantic diagnostics are included but separable ──────────────

    #[test]
    fn semantic_diagnostics_present_for_unknown_table() {
        let source = r#"db.execute(f"SELECT id FROM unknown_tbl")"#;
        let all = analyzer().validate(&extract_python(source));
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
        let source = r#"conn.execute(f"INSERT INTO orders (a, b) VALUS ({x}, {y})")"#;
        let fragments = extract_python(source);
        assert_eq!(fragments.len(), 1);
        let parse_diags: Vec<_> = analyzer()
            .validate(&fragments)
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
        let source = r#"db.execute(f"INSERT INTO t (a) VALUES (datetime('now'))")"#;
        let unknown_fn: Vec<_> = analyzer()
            .validate(&extract_python(source))
            .into_iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownFunction { .. }))
            .collect();
        assert!(
            unknown_fn.is_empty(),
            "datetime should not be flagged as unknown, got: {unknown_fn:?}",
        );
    }

    #[test]
    fn semantic_tokens_classify_function_callee() {
        let source = r#"db.execute(f"INSERT INTO t (a) VALUES (datetime('now'))")"#;
        let fragments = extract_python(source);
        assert_eq!(fragments.len(), 1);
        let tokens =
            EmbeddedAnalyzer::new(crate::dialect::sqlite()).fragment_semantic_tokens(&fragments[0]);
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
        let source = r#"db.execute(f"SELECT {col} FROM {tbl}")"#;
        let all = analyzer().validate(&extract_python(source));
        for diag in &all {
            let msg = format!("{}", diag.message);
            assert!(
                !msg.contains("__hole_"),
                "hole placeholder leaked into diagnostics: {msg}",
            );
        }
    }
}

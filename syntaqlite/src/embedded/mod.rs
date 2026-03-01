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
//! - [`python`] — Python f-string extraction
//! - [`typescript`] — TypeScript/JavaScript template literal extraction

pub mod offset_map;
mod python;
mod typescript;

pub use python::extract_python;
pub use typescript::extract_typescript;

use std::ops::Range;

use crate::Dialect;
use crate::parser::{LowLevelParser, Tokenizer};
use crate::validation::{Diagnostic, DiagnosticMessage, ValidationConfig, validate_document};

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
    "SELECT", "INSERT", "UPDATE", "DELETE", "CREATE", "ALTER", "DROP", "WITH",
    "EXPLAIN", "PRAGMA", "ATTACH", "DETACH", "REINDEX", "VACUUM", "BEGIN",
    "COMMIT", "ROLLBACK", "SAVEPOINT", "RELEASE",
];

/// Check if the given text starts with a SQL keyword (case-insensitive).
fn starts_with_sql_keyword(text: &str) -> bool {
    let trimmed = text.trim_start();
    for kw in SQL_KEYWORDS {
        if trimmed.len() >= kw.len()
            && trimmed[..kw.len()].eq_ignore_ascii_case(kw)
            && (trimmed.len() == kw.len()
                || !trimmed.as_bytes()[kw.len()].is_ascii_alphanumeric())
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
/// Returns diagnostics with offsets mapped back to host-file positions.
pub fn validate_embedded(
    dialect: &Dialect,
    fragments: &[EmbeddedFragment],
    config: &ValidationConfig,
) -> Vec<Diagnostic> {
    let mut all_diags = Vec::new();

    for fragment in fragments {
        let diags = validate_fragment(dialect, fragment, config);
        let offset_map = OffsetMap::new(fragment);

        for mut d in diags {
            if is_hole_diagnostic(&d, fragment) {
                continue;
            }
            d.start_offset = offset_map.to_host(d.start_offset);
            d.end_offset = offset_map.to_host(d.end_offset);
            all_diags.push(d);
        }
    }

    all_diags
}

/// Tokenize, parse (with hole wrapping), and validate a single fragment.
///
/// Returns diagnostics with SQL-text byte offsets (not yet mapped to host).
fn validate_fragment(
    dialect: &Dialect,
    fragment: &EmbeddedFragment,
    config: &ValidationConfig,
) -> Vec<Diagnostic> {
    let mut parser = LowLevelParser::with_dialect(dialect);
    let mut tokenizer = Tokenizer::with_dialect(*dialect);

    // Tokenize the processed SQL text.
    let tokens: Vec<(u32, usize, usize)> = {
        let cursor = tokenizer.tokenize(&fragment.sql_text);
        cursor
            .map(|tok| {
                let offset =
                    tok.text.as_ptr() as usize - fragment.sql_text.as_ptr() as usize;
                (tok.token_type, offset, tok.text.len())
            })
            .collect()
    };

    // Feed tokens to the low-level parser.
    let mut cursor = parser.feed(&fragment.sql_text);
    let mut stmt_ids = Vec::new();

    for &(token_type, offset, length) in &tokens {
        let hole = fragment.holes.iter().find(|h| {
            offset >= h.sql_offset && offset < h.sql_offset + h.placeholder.len()
        });

        if let Some(hole) = hole {
            cursor.begin_macro(hole.host_range.start as u32, hole.host_range.len() as u32);
        }

        match cursor.feed_token(token_type, offset..offset + length) {
            Ok(Some(root)) => stmt_ids.push(root),
            Ok(None) => {}
            Err(e) => {
                if let Some(root) = e.root {
                    stmt_ids.push(root);
                }
            }
        }

        if hole.is_some() {
            cursor.end_macro();
        }
    }

    match cursor.finish() {
        Ok(Some(root)) => stmt_ids.push(root),
        Ok(None) => {}
        Err(e) => {
            if let Some(root) = e.root {
                stmt_ids.push(root);
            }
        }
    }

    // After finish(), the cursor's reader still points at valid arena data.
    validate_document(cursor.base().reader(), &stmt_ids, dialect, None, &[], config)
}

/// Check if a diagnostic message references a hole placeholder name.
fn is_hole_diagnostic(diag: &Diagnostic, fragment: &EmbeddedFragment) -> bool {
    match &diag.message {
        DiagnosticMessage::UnknownTable { name }
        | DiagnosticMessage::UnknownFunction { name } => {
            fragment.holes.iter().any(|h| h.placeholder == *name)
        }
        DiagnosticMessage::UnknownColumn { column, .. } => {
            fragment.holes.iter().any(|h| h.placeholder == *column)
        }
        _ => false,
    }
}

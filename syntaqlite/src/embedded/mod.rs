// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Embedded SQL extraction from host language sources (e.g. Python f-strings).
//!
//! Extracts SQL fragments from host language files, replaces interpolation holes
//! with placeholder identifiers, parses the SQL with `begin_macro`/`end_macro`
//! wrapping each hole, runs validation, and maps diagnostic offsets back to
//! host-file positions.

pub mod offset_map;

use std::ops::Range;

use crate::Dialect;
use crate::parser::{LowLevelParser, Tokenizer};
use crate::validation::{Diagnostic, DiagnosticMessage, ValidationConfig, validate_document};

use offset_map::OffsetMap;

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

/// An interpolation hole (e.g. `{expr}` in a Python f-string).
#[derive(Debug)]
pub struct Hole {
    /// Byte range of `{expr}` in the host file.
    pub host_range: Range<usize>,
    /// Byte offset in `sql_text` where the placeholder sits.
    pub sql_offset: usize,
    /// The placeholder identifier (e.g. `__hole_0__`).
    pub placeholder: String,
}

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

// ── Python f-string extraction ──────────────────────────────────────────

/// Extract SQL fragments from Python source code.
///
/// Scans for f-strings (`f"..."`, `f'...'`, `f"""..."""`, `f'''...'''`) and
/// checks if their content starts with a SQL keyword. For qualifying strings,
/// interpolation holes (`{expr}`) are replaced with `__hole_N__` placeholders.
pub fn extract_python(source: &str) -> Vec<EmbeddedFragment> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut fragments = Vec::new();
    let mut i = 0;

    while i < len {
        let Some((quote_char, quote_len, content_start)) = detect_fstring_prefix(bytes, i) else {
            i += 1;
            continue;
        };

        let Some((content_end, string_end)) =
            find_string_end(bytes, content_start, quote_char, quote_len)
        else {
            i = content_start;
            continue;
        };

        let content = &source[content_start..content_end];
        if starts_with_sql_keyword(content) {
            if let Some(fragment) = extract_fstring_fragment(source, content_start, content_end) {
                fragments.push(fragment);
            }
        }

        i = string_end;
    }

    fragments
}

/// Detect an f-string prefix at position `i`. Returns `(quote_char, quote_len, content_start)`.
/// `quote_len` is 1 for single-quoted, 3 for triple-quoted.
fn detect_fstring_prefix(bytes: &[u8], i: usize) -> Option<(u8, usize, usize)> {
    let len = bytes.len();
    let (has_f, prefix_len) = if i + 1 < len && (bytes[i] == b'f' || bytes[i] == b'F') {
        if bytes[i + 1] == b'"' || bytes[i + 1] == b'\'' {
            (true, 1)
        } else if i + 2 < len
            && (bytes[i + 1] == b'r' || bytes[i + 1] == b'R')
            && (bytes[i + 2] == b'"' || bytes[i + 2] == b'\'')
        {
            (true, 2)
        } else {
            (false, 0)
        }
    } else if i + 2 < len
        && (bytes[i] == b'r' || bytes[i] == b'R')
        && (bytes[i + 1] == b'f' || bytes[i + 1] == b'F')
        && (bytes[i + 2] == b'"' || bytes[i + 2] == b'\'')
    {
        (true, 2)
    } else {
        (false, 0)
    };

    if !has_f {
        return None;
    }

    let quote_pos = i + prefix_len;
    let quote_char = bytes[quote_pos];

    if quote_pos + 2 < len
        && bytes[quote_pos + 1] == quote_char
        && bytes[quote_pos + 2] == quote_char
    {
        Some((quote_char, 3, quote_pos + 3))
    } else {
        Some((quote_char, 1, quote_pos + 1))
    }
}

/// Find the end of a string literal. Returns `(content_end, string_end)`.
fn find_string_end(
    bytes: &[u8],
    start: usize,
    quote_char: u8,
    quote_len: usize,
) -> Option<(usize, usize)> {
    let len = bytes.len();
    let mut j = start;
    while j < len {
        if bytes[j] == b'\\' {
            j += 2;
            continue;
        }
        if bytes[j] == quote_char {
            if quote_len == 3 {
                if j + 2 < len
                    && bytes[j + 1] == quote_char
                    && bytes[j + 2] == quote_char
                {
                    return Some((j, j + 3));
                }
            } else {
                return Some((j, j + 1));
            }
        }
        if quote_len == 1 && bytes[j] == b'\n' {
            return None;
        }
        j += 1;
    }
    None
}

/// Extract a single f-string fragment, processing `{expr}` holes.
fn extract_fstring_fragment(
    source: &str,
    content_start: usize,
    content_end: usize,
) -> Option<EmbeddedFragment> {
    let bytes = source.as_bytes();
    let mut sql_text = String::new();
    let mut holes = Vec::new();
    let mut j = content_start;
    let mut hole_idx = 0;

    while j < content_end {
        if bytes[j] == b'{' {
            if j + 1 < content_end && bytes[j + 1] == b'{' {
                sql_text.push('{');
                j += 2;
                continue;
            }

            let hole_start = j;
            let hole_content_end = find_matching_brace(bytes, j + 1, content_end)?;
            let hole_end = hole_content_end + 1;

            let placeholder = format!("__hole_{hole_idx}__");
            let sql_offset = sql_text.len();
            sql_text.push_str(&placeholder);

            holes.push(Hole {
                host_range: hole_start..hole_end,
                sql_offset,
                placeholder,
            });

            hole_idx += 1;
            j = hole_end;
        } else if bytes[j] == b'}' && j + 1 < content_end && bytes[j + 1] == b'}' {
            sql_text.push('}');
            j += 2;
        } else {
            sql_text.push(bytes[j] as char);
            j += 1;
        }
    }

    Some(EmbeddedFragment {
        sql_range: content_start..content_end,
        sql_text,
        holes,
    })
}

/// Find the matching `}` for an f-string interpolation hole.
fn find_matching_brace(bytes: &[u8], start: usize, end: usize) -> Option<usize> {
    let mut depth = 1u32;
    let mut j = start;

    while j < end {
        match bytes[j] {
            b'{' => {
                depth += 1;
                j += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
                j += 1;
            }
            b'\'' | b'"' => {
                j = skip_python_string(bytes, j, end);
            }
            b'#' => {
                while j < end && bytes[j] != b'\n' {
                    j += 1;
                }
            }
            _ => j += 1,
        }
    }
    None
}

/// Skip past a Python string literal starting at `pos`.
fn skip_python_string(bytes: &[u8], pos: usize, end: usize) -> usize {
    let quote = bytes[pos];
    let mut j = pos + 1;

    if j + 1 < end && bytes[j] == quote && bytes[j + 1] == quote {
        j += 2;
        while j < end {
            if bytes[j] == b'\\' {
                j += 2;
                continue;
            }
            if j + 2 < end
                && bytes[j] == quote
                && bytes[j + 1] == quote
                && bytes[j + 2] == quote
            {
                return j + 3;
            }
            j += 1;
        }
        return j;
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_simple_fstring() {
        let source = r#"query = f"SELECT * FROM users WHERE id = {uid}""#;
        let fragments = extract_python(source);
        assert_eq!(fragments.len(), 1);

        let f = &fragments[0];
        assert_eq!(f.holes.len(), 1);
        assert_eq!(f.holes[0].placeholder, "__hole_0__");
        assert!(f.sql_text.contains("__hole_0__"));
        assert!(f.sql_text.starts_with("SELECT * FROM users WHERE id = "));
    }

    #[test]
    fn extract_escaped_braces() {
        let source = r#"s = f"SELECT '{{literal}}' FROM t""#;
        let fragments = extract_python(source);
        assert_eq!(fragments.len(), 1);
        assert_eq!(fragments[0].holes.len(), 0);
        assert!(fragments[0].sql_text.contains("{literal}"));
    }

    #[test]
    fn extract_multiple_holes() {
        let source = r#"q = f"SELECT {cols} FROM {table} WHERE {col} = {val}""#;
        let fragments = extract_python(source);
        assert_eq!(fragments.len(), 1);
        assert_eq!(fragments[0].holes.len(), 4);
    }

    #[test]
    fn skip_non_sql_fstring() {
        let source = r#"msg = f"Hello {name}, welcome!""#;
        let fragments = extract_python(source);
        assert_eq!(fragments.len(), 0);
    }

    #[test]
    fn extract_triple_quoted() {
        let source = "q = f\"\"\"\nSELECT *\nFROM users\nWHERE id = {uid}\n\"\"\"";
        let fragments = extract_python(source);
        assert_eq!(fragments.len(), 1);
        assert_eq!(fragments[0].holes.len(), 1);
    }

    #[test]
    fn extract_rf_prefix() {
        let source = r#"q = rf"SELECT * FROM users WHERE id = {uid}""#;
        let fragments = extract_python(source);
        assert_eq!(fragments.len(), 1);
    }

    #[test]
    fn extract_fr_prefix() {
        let source = r#"q = fr"SELECT * FROM users WHERE id = {uid}""#;
        let fragments = extract_python(source);
        assert_eq!(fragments.len(), 1);
    }
}

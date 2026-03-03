// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Keyword table parsing from `mkkeywordhash.c`.
//!
//! Parses the keyword array and mask `#define` blocks to produce a
//! structured representation of the keyword table for a given SQLite version.

/// A single keyword entry from the keyword table array.
#[derive(Debug, Clone)]
pub(crate) struct KeywordEntry {
    /// The keyword name, e.g. "RETURNING".
    pub(crate) name: String,
    /// The token constant, e.g. "TK_RETURNING".
    pub(crate) token: String,
    /// The mask expression (symbol names ORed together), e.g. "RETURNING".
    pub(crate) mask_expr: String,
    /// Priority value.
    pub(crate) priority: u32,
}

/// A mask `#define` block mapping an SQLITE_OMIT_* or SQLITE_ENABLE_* flag to a bitmask value.
#[derive(Debug, Clone)]
pub(crate) struct MaskDefine {
    /// The mask symbol name, e.g. "RETURNING".
    pub(crate) name: String,
    /// The SQLITE_OMIT_* or SQLITE_ENABLE_* flag, e.g. "SQLITE_OMIT_RETURNING".
    pub(crate) omit_flag: String,
    /// Polarity: 0 = OMIT (keyword removed when flag set), 1 = ENABLE (keyword added when flag set).
    pub(crate) polarity: u8,
}

/// The full keyword table for a single SQLite version.
#[derive(Debug, Clone)]
pub(crate) struct KeywordTable {
    pub(crate) keywords: Vec<KeywordEntry>,
    pub(crate) masks: Vec<MaskDefine>,
}

/// Parse the keyword table and mask defines from `mkkeywordhash.c` source.
pub(crate) fn parse_keyword_table(source: &str) -> Result<KeywordTable, String> {
    let keywords = parse_keyword_array(source)?;
    let masks = parse_mask_defines(source);
    Ok(KeywordTable { keywords, masks })
}

/// Parse entries from the `aKeywordTable[]` array.
///
/// Each entry looks like:
/// ```c
///   { "ABORT",            "TK_ABORT",        CONFLICT|TRIGGER, 0      },
/// ```
fn parse_keyword_array(source: &str) -> Result<Vec<KeywordEntry>, String> {
    let mut entries = Vec::new();
    let mut in_table = false;

    for line in source.lines() {
        let trimmed = line.trim();

        // Detect start of the keyword table array.
        if trimmed.contains("aKeywordTable[]") && trimmed.contains('{') {
            in_table = true;
            continue;
        }

        if !in_table {
            continue;
        }

        // End of array.
        if trimmed == "};" {
            break;
        }

        // Parse a table entry: { "NAME", "TK_NAME", MASK_EXPR, PRIORITY },
        if let Some(entry) = parse_keyword_line(trimmed) {
            entries.push(entry);
        }
    }

    if entries.is_empty() {
        return Err("no keyword entries found in aKeywordTable".to_string());
    }

    Ok(entries)
}

fn parse_keyword_line(line: &str) -> Option<KeywordEntry> {
    // Strip leading/trailing braces and comma.
    let line = line
        .trim()
        .trim_start_matches('{')
        .trim_end_matches([',', '}']);
    let line = line.trim();

    // Split by commas, handling that mask_expr may contain `|`.
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                current.push(ch);
            }
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(ch),
        }
    }
    fields.push(current.trim().to_string());

    // Pre-3.31 has 3 fields (no priority), 3.31+ has 4 fields.
    if fields.len() < 3 {
        return None;
    }

    let name = fields[0].trim_matches('"').to_string();
    let token = fields[1].trim_matches('"');
    let mask_expr = fields[2].clone();

    if name.is_empty() || !token.starts_with("TK_") {
        return None;
    }

    Some(KeywordEntry {
        name,
        token: token.to_string(),
        mask_expr,
        priority: if fields.len() >= 4 {
            fields[3].trim().parse().unwrap_or(0)
        } else {
            0
        },
    })
}

/// Parse mask `#define` blocks.
///
/// Handles two patterns:
///
/// OMIT pattern (polarity=0):
/// ```c
/// #ifdef SQLITE_OMIT_RETURNING
/// #  define RETURNING  0
/// #else
/// #  define RETURNING  0x00400000
/// #endif
/// ```
///
/// ENABLE pattern (polarity=1):
/// ```c
/// #ifndef SQLITE_ENABLE_ORDERED_SET_AGGREGATES
/// #  define ORDERSET   0
/// #else
/// #  define ORDERSET   0x00800000
/// #endif
/// ```
fn parse_mask_defines(source: &str) -> Vec<MaskDefine> {
    let mut masks = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        // OMIT pattern: #ifdef SQLITE_OMIT_*
        if let Some(rest) = trimmed.strip_prefix("#ifdef ") {
            let flag = rest.trim();
            if flag.starts_with("SQLITE_OMIT_")
                && let Some(mask) = parse_ifdef_mask_block(&lines[i..], flag, 0)
            {
                masks.push(mask);
            }
        }

        // ENABLE pattern: #ifndef SQLITE_ENABLE_*
        if let Some(rest) = trimmed.strip_prefix("#ifndef ") {
            let flag = rest.trim();
            if flag.starts_with("SQLITE_ENABLE_") {
                // For #ifndef SQLITE_ENABLE_*, the non-zero value is in the #else branch
                // (same position as OMIT), but polarity is inverted.
                if let Some(mask) = parse_ifdef_mask_block(&lines[i..], flag, 1) {
                    masks.push(mask);
                }
            }
        }

        i += 1;
    }

    masks
}

fn parse_ifdef_mask_block(lines: &[&str], flag: &str, polarity: u8) -> Option<MaskDefine> {
    // Scan for #else, then find the #define with a hex value.
    let mut in_else = false;
    let mut name = None;
    let mut bit_value = None;

    for line in lines.iter().skip(1) {
        let trimmed = line.trim();

        if trimmed.starts_with("#else") {
            in_else = true;
            continue;
        }

        if trimmed.starts_with("#endif") {
            break;
        }

        if in_else
            && let Some(rest) = trimmed
                .strip_prefix("#")
                .and_then(|s| s.trim_start().strip_prefix("define"))
        {
            let rest = rest.trim_start();
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                name = Some(parts[0].to_string());
                if let Some(hex) = parts[1].strip_prefix("0x") {
                    bit_value = u64::from_str_radix(hex, 16).ok();
                }
            }
        }
    }

    // Require a valid hex bit value to confirm this is a real mask block.
    let _ = bit_value?;
    Some(MaskDefine {
        name: name?,
        omit_flag: flag.to_string(),
        polarity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_keyword_line_basic() {
        let entry = parse_keyword_line(
            r#"  { "ABORT",            "TK_ABORT",        CONFLICT|TRIGGER, 0      },"#,
        )
        .unwrap();
        assert_eq!(entry.name, "ABORT");
        assert_eq!(entry.mask_expr, "CONFLICT|TRIGGER");
    }

    #[test]
    fn parse_mask_block_omit() {
        let source = r#"
#ifdef SQLITE_OMIT_RETURNING
#  define RETURNING  0
#else
#  define RETURNING  0x00400000
#endif
"#;
        let masks = parse_mask_defines(source);
        assert_eq!(masks.len(), 1);
        assert_eq!(masks[0].name, "RETURNING");
        assert_eq!(masks[0].omit_flag, "SQLITE_OMIT_RETURNING");
        assert_eq!(masks[0].polarity, 0);
    }

    #[test]
    fn parse_mask_block_enable() {
        let source = r#"
#ifndef SQLITE_ENABLE_ORDERED_SET_AGGREGATES
#  define ORDERSET   0
#else
#  define ORDERSET   0x00800000
#endif
"#;
        let masks = parse_mask_defines(source);
        assert_eq!(masks.len(), 1);
        assert_eq!(masks[0].name, "ORDERSET");
        assert_eq!(masks[0].omit_flag, "SQLITE_ENABLE_ORDERED_SET_AGGREGATES");
        assert_eq!(masks[0].polarity, 1);
    }

    #[test]
    fn parse_mixed_omit_and_enable() {
        let source = r#"
#ifdef SQLITE_OMIT_WINDOWFUNC
#  define WINDOWFUNC 0
#else
#  define WINDOWFUNC 0x00100000
#endif
#ifdef SQLITE_OMIT_RETURNING
#  define RETURNING  0
#else
#  define RETURNING  0x00400000
#endif
#ifndef SQLITE_ENABLE_ORDERED_SET_AGGREGATES
#  define ORDERSET   0
#else
#  define ORDERSET   0x00800000
#endif
"#;
        let masks = parse_mask_defines(source);
        assert_eq!(masks.len(), 3);
        assert_eq!(masks[0].name, "WINDOWFUNC");
        assert_eq!(masks[0].polarity, 0);
        assert_eq!(masks[1].name, "RETURNING");
        assert_eq!(masks[1].polarity, 0);
        assert_eq!(masks[2].name, "ORDERSET");
        assert_eq!(masks[2].polarity, 1);
    }
}

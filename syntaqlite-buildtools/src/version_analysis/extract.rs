// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Fragment extraction from SQLite source files.
//!
//! Reuses `CExtractor` to pull the same fragments that the main codegen
//! pipeline extracts, but returns them as raw strings for hashing/diffing
//! rather than feeding them into the C transformer.

use crate::util::c_source::c_extractor::CExtractor;

/// The names of extractable fragments (excluding keywords, handled separately).
pub const FRAGMENT_NAMES: &[&str] = &[
    "cc_defines",
    "ai_class",
    "id_char",
    "char_map",
    "get_token",
    "ctype_map",
    "upper_to_lower",
    "is_macros",
];

/// CC_* define names to extract (same list as `sqlite_runtime_codegen.rs`).
const CC_DEFINE_NAMES: &[&str] = &[
    "CC_X",
    "CC_KYWD0",
    "CC_KYWD",
    "CC_DIGIT",
    "CC_DOLLAR",
    "CC_VARALPHA",
    "CC_VARNUM",
    "CC_SPACE",
    "CC_QUOTE",
    "CC_QUOTE2",
    "CC_PIPE",
    "CC_MINUS",
    "CC_LT",
    "CC_GT",
    "CC_EQ",
    "CC_BANG",
    "CC_SLASH",
    "CC_LP",
    "CC_RP",
    "CC_SEMI",
    "CC_PLUS",
    "CC_STAR",
    "CC_PERCENT",
    "CC_COMMA",
    "CC_AND",
    "CC_TILDA",
    "CC_DOT",
    "CC_ID",
    "CC_ILLEGAL",
    "CC_NUL",
    "CC_BOM",
];

/// All extracted fragments from a single SQLite version's source files.
#[derive(Debug)]
pub struct ExtractedFragments {
    pub cc_defines: Result<String, String>,
    pub ai_class: Result<String, String>,
    pub id_char: Result<String, String>,
    pub char_map: Result<String, String>,
    pub get_token: Result<String, String>,
    pub ctype_map: Result<String, String>,
    pub upper_to_lower: Result<String, String>,
    pub is_macros: Result<String, String>,
}

impl ExtractedFragments {
    /// Look up a fragment by name.
    pub fn get(&self, name: &str) -> Result<&str, String> {
        let field = match name {
            "cc_defines" => &self.cc_defines,
            "ai_class" => &self.ai_class,
            "id_char" => &self.id_char,
            "char_map" => &self.char_map,
            "get_token" => &self.get_token,
            "ctype_map" => &self.ctype_map,
            "upper_to_lower" => &self.upper_to_lower,
            "is_macros" => &self.is_macros,
            _ => return Err(format!("unknown fragment: {name}")),
        };
        field.as_deref().map_err(|e| e.clone())
    }
}

/// Extract all code fragments from a single SQLite version's source files.
///
/// Individual extraction failures are captured per-field rather than
/// short-circuiting, so the caller can report partial results.
pub fn extract_fragments(
    tokenize_c: &str,
    global_c: &str,
    sqliteint_h: &str,
) -> Result<ExtractedFragments, String> {
    let tok = CExtractor::new(tokenize_c);
    let glob = CExtractor::new(global_c);
    let sqint = CExtractor::new(sqliteint_h);

    Ok(ExtractedFragments {
        cc_defines: tok
            .extract_specific_defines(CC_DEFINE_NAMES)
            .map(|d| d.text),
        ai_class: tok.extract_static_array("aiClass").map(|a| a.text),
        id_char: tok
            .extract_defines_with_ifdef_context(&["IdChar"])
            .map(|d| d.text),
        char_map: tok
            .extract_defines_with_ifdef_context(&["charMap"])
            .map(|d| d.text),
        get_token: tok.extract_function("sqlite3GetToken").map(|f| f.text),
        ctype_map: glob.extract_static_array("sqlite3CtypeMap").map(|a| a.text),
        upper_to_lower: glob
            .extract_static_array("sqlite3UpperToLower")
            .map(|a| a.text),
        is_macros: sqint
            .extract_specific_defines(&["sqlite3Isspace", "sqlite3Isdigit", "sqlite3Isxdigit"])
            .map(|d| d.text),
    })
}

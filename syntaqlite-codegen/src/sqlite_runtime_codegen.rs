// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fs;

use crate::c_source::{c_extractor, c_transformer};
use crate::util::pascal_case;
use crate::writers::c_writer::CWriter;

use crate::TokenizerExtractResult;

pub(crate) fn extract_tokenizer(
    tokenize_content: &str,
    global_content: &str,
    sqliteint_content: &str,
    dialect: &str,
) -> Result<(String, TokenizerExtractResult), String> {
    let tokenize_extractor = c_extractor::CExtractor::new(tokenize_content);

    let global_extractor = c_extractor::CExtractor::new(global_content);

    let sqliteint_extractor = c_extractor::CExtractor::new(sqliteint_content);

    let cc_defines = tokenize_extractor.extract_specific_defines(&[
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
    ])?;
    let ai_class = tokenize_extractor.extract_static_array("aiClass")?;
    let ctype_map = global_extractor.extract_static_array("sqlite3CtypeMap")?;
    let upper_to_lower = global_extractor.extract_static_array("sqlite3UpperToLower")?;
    let is_macros = sqliteint_extractor.extract_specific_defines(&[
        "sqlite3Isspace",
        "sqlite3Isdigit",
        "sqlite3Isxdigit",
    ])?;
    let id_char = tokenize_extractor.extract_defines_with_ifdef_context(&["IdChar"])?;
    let char_map = tokenize_extractor.extract_defines_with_ifdef_context(&["charMap"])?;
    let function = tokenize_extractor.extract_function("sqlite3GetToken")?;

    let combined = {
        let mut w = CWriter::new();
        w.sqlite_file_header();
        w.include_local("syntaqlite_ext/sqlite_compat.h");
        w.include_local(&format!("syntaqlite_{dialect}/{dialect}_tokens.h"));
        w.include_local("csrc/sqlite_keyword.h")
            .newline()
            .fragment(&cc_defines)
            .newline()
            .fragment(&ctype_map)
            .newline()
            .fragment(&is_macros)
            .newline()
            .fragment(&id_char)
            .newline()
            .fragment(&ai_class)
            .newline()
            .fragment(&function);
        w.finish()
    };

    let get_token_name = format!("Synq{}GetToken", pascal_case(dialect));
    let get_token_base = format!("{}_base", get_token_name);
    let output = c_transformer::CTransformer::new(&combined)
        .add_array_static("sqlite3CtypeMap")
        .insert_after_includes("#include \"syntaqlite/dialect_config.h\"")
        .replace_in_function(
            "sqlite3GetToken",
            "keywordCode((char*)",
            "synq_sqlite3_keywordCode(config, (char*)",
        )
        .replace_in_function(
            "sqlite3GetToken",
            "sqlite3GetToken(const unsigned char *z",
            "sqlite3GetToken(const SyntaqliteDialectConfig* config, const unsigned char *z",
        )
        .rename_function("sqlite3GetToken", &get_token_base)
        // Make the base function static (internal linkage only).
        .replace_all(
            &format!("i64 {}(", get_token_base),
            &format!("static i64 {}(", get_token_base),
        )
        .replace_all("TK_", "SYNTAQLITE_TK_")
        .append(&generate_get_token_wrapper(
            &get_token_name,
            &get_token_base,
        ))
        .finish();

    Ok((
        output,
        TokenizerExtractResult {
            char_map: char_map.text,
            upper_to_lower: upper_to_lower.text,
        },
    ))
}

/// Generate the public `GetToken` wrapper that calls the `_base` function then
/// applies version-dependent token reclassification (the "postlude").
///
/// When `sqlite_version` is `INT32_MAX` (latest), the postlude is a single
/// branch-not-taken. When compiled with `SYNQ_SQLITE_VERSION` defined, the
/// compiler constant-folds and eliminates dead branches.
fn generate_get_token_wrapper(public_name: &str, base_name: &str) -> String {
    format!(
        r#"
i64 {public_name}(const SyntaqliteDialectConfig* config, const unsigned char *z, int *tokenType){{
  i64 len = {base_name}(config, z, tokenType);
  /* Version-dependent token reclassification. */
  if( SYNQ_VER_LT(config, 3038000) && *tokenType==SYNTAQLITE_TK_PTR ){{
    /* -> and ->> operators added in 3.38.
    ** Return just the '-' as TK_MINUS; next call picks up '>' naturally. */
    *tokenType = SYNTAQLITE_TK_MINUS;
    return 1;
  }}
  if( SYNQ_VER_LT(config, 3046000) && *tokenType==SYNTAQLITE_TK_QNUMBER ){{
    /* Digit separators added in 3.46.
    ** Truncate to the first underscore. */
    i64 j;
    int saw_dot = 0;
    for(j=0; j<len; j++){{
      if( z[j]=='_' ) break;
      if( z[j]=='.' ) saw_dot = 1;
    }}
    *tokenType = saw_dot ? SYNTAQLITE_TK_FLOAT : SYNTAQLITE_TK_INTEGER;
    return j;
  }}
  return len;
}}
"#
    )
}

/// Extract terminal symbols (potential keywords) from extension `.y` grammar files.
///
/// Collects from `%token`, `%fallback`, and rule RHS symbols.
///
/// `ID` is intentionally excluded: it is a parser identifier token, not an SQL
/// keyword, and including it would cause identifier tokens to be misclassified
/// as keywords in semantic highlighting metadata.
pub(crate) fn extract_terminals_from_y(extension_y_contents: &[&str]) -> Vec<String> {
    use std::collections::HashSet;

    let mut terminals: HashSet<String> = HashSet::new();

    for content in extension_y_contents {
        let grammar = match crate::grammar_parser::LemonGrammar::parse(content) {
            Ok(g) => g,
            Err(_) => continue,
        };

        for tok in &grammar.tokens {
            if is_keyword_like(tok.name) {
                terminals.insert(tok.name.to_string());
            }
        }

        for fb in &grammar.fallbacks {
            for tok in &fb.tokens {
                if is_keyword_like(tok) {
                    terminals.insert(tok.to_string());
                }
            }
        }

        for rule in &grammar.rules {
            for sym in &rule.rhs {
                if is_keyword_like(sym.name) && sym.name != "ID" {
                    terminals.insert(sym.name.to_string());
                }
            }
        }
    }

    terminals.into_iter().collect()
}

fn is_keyword_like(name: &str) -> bool {
    name.len() >= 2 && name.chars().all(|c| c.is_ascii_uppercase())
}

/// Parse `testcase(i==N); /* NAME */` comments from raw mkkeywordhash output
/// to get (1-based index, keyword name) pairs.
fn parse_keyword_indices(code: &str) -> Vec<(usize, String)> {
    let mut result = Vec::new();
    for line in code.lines() {
        let trimmed = line.trim();
        // Format: "testcase( i==N ); /* NAME */"
        let Some(rest) = trimmed.strip_prefix("testcase( i==") else {
            continue;
        };
        let Some(space_pos) = rest.find(' ') else {
            continue;
        };
        let Ok(idx) = rest[..space_pos].parse::<usize>() else {
            continue;
        };
        let Some(cs) = rest.find("/* ") else {
            continue;
        };
        let after = &rest[cs + 3..];
        let Some(ce) = after.find(" */") else {
            continue;
        };
        result.push((idx, after[..ce].to_string()));
    }
    result
}

/// Return the SQLite version (as integer, e.g. 3035000) in which a keyword was
/// first introduced.  Returns 0 for baseline keywords (present in 3.12.2).
fn keyword_since_version(name: &str) -> i32 {
    match name {
        "DO" | "NOTHING" => 3024000,
        "CURRENT" | "FILTER" | "FOLLOWING" | "OVER" | "PARTITION" | "PRECEDING" | "RANGE"
        | "ROWS" | "UNBOUNDED" | "WINDOW" => 3025000,
        "EXCLUDE" | "GROUPS" | "OTHERS" | "TIES" => 3028000,
        "FIRST" | "LAST" | "NULLS" => 3030000,
        "ALWAYS" | "GENERATED" => 3031000,
        "MATERIALIZED" | "RETURNING" => 3035000,
        "WITHIN" => 3047000,
        _ => 0,
    }
}

/// SYNQ cflag constants, mirroring `sqlite_cflags.h`.
const SYNQ_CFLAG_TABLE: &[(&str, u32)] = &[
    ("SYNQ_SQLITE_OMIT_EXPLAIN", 0x00000001),
    ("SYNQ_SQLITE_OMIT_TEMPDB", 0x00000002),
    ("SYNQ_SQLITE_OMIT_COMPOUND_SELECT", 0x00000004),
    ("SYNQ_SQLITE_OMIT_WINDOWFUNC", 0x00000008),
    ("SYNQ_SQLITE_OMIT_GENERATED_COLUMNS", 0x00000010),
    ("SYNQ_SQLITE_OMIT_VIEW", 0x00000020),
    ("SYNQ_SQLITE_OMIT_CTE", 0x00000040),
    ("SYNQ_SQLITE_OMIT_SUBQUERY", 0x00000080),
    ("SYNQ_SQLITE_OMIT_CAST", 0x00000100),
    ("SYNQ_SQLITE_OMIT_PRAGMA", 0x00000200),
    ("SYNQ_SQLITE_OMIT_TRIGGER", 0x00000400),
    ("SYNQ_SQLITE_OMIT_ATTACH", 0x00000800),
    ("SYNQ_SQLITE_OMIT_REINDEX", 0x00001000),
    ("SYNQ_SQLITE_OMIT_ANALYZE", 0x00002000),
    ("SYNQ_SQLITE_OMIT_ALTERTABLE", 0x00004000),
    ("SYNQ_SQLITE_OMIT_VIRTUALTABLE", 0x00008000),
    ("SYNQ_SQLITE_OMIT_RETURNING", 0x00010000),
    ("SYNQ_SQLITE_ENABLE_ORDERED_SET_AGGREGATES", 0x00020000),
];

/// Look up the SYNQ cflag value for a `SQLITE_OMIT_*` or `SQLITE_ENABLE_*` flag.
fn synq_cflag_for_sqlite_flag(sqlite_flag: &str) -> Option<u32> {
    let synq_name = format!("SYNQ_{sqlite_flag}");
    SYNQ_CFLAG_TABLE
        .iter()
        .find(|(name, _)| *name == synq_name)
        .map(|(_, val)| *val)
}

/// Parsed cflag data for a keyword: (cflag_value, polarity).
struct KeywordCflagMap {
    /// Map from keyword name → (cflag_value, polarity).
    map: std::collections::HashMap<String, (u32, u8)>,
}

impl KeywordCflagMap {
    /// Build cflag map by parsing the embedded `mkkeywordhash.c` source.
    fn from_mkkeywordhash() -> Self {
        use crate::mkkeywordhash_parser;

        let source = crate::embedded_mkkeywordhash_c();
        let table = mkkeywordhash_parser::parse_keyword_table(source)
            .expect("failed to parse embedded mkkeywordhash.c");

        // Build mask_name → (omit_flag, polarity) lookup.
        let mask_lookup: std::collections::HashMap<&str, (&str, u8)> = table
            .masks
            .iter()
            .map(|m| (m.name.as_str(), (m.omit_flag.as_str(), m.polarity)))
            .collect();

        let mut map = std::collections::HashMap::new();

        for kw in &table.keywords {
            // Skip keywords with ALWAYS mask (value 0x00000002) — always enabled.
            if kw.mask_expr == "ALWAYS" {
                continue;
            }

            // Skip OR'd masks (e.g. "CONFLICT|TRIGGER") — no single SYNQ constant.
            if kw.mask_expr.contains('|') {
                continue;
            }

            // Look up the mask symbol in the defines.
            if let Some(&(omit_flag, polarity)) = mask_lookup.get(kw.mask_expr.as_str()) {
                if let Some(cflag_val) = synq_cflag_for_sqlite_flag(omit_flag) {
                    map.insert(kw.name.clone(), (cflag_val, polarity));
                }
            }
        }

        Self { map }
    }

    fn get(&self, keyword: &str) -> Option<(u32, u8)> {
        self.map.get(keyword).copied()
    }
}

/// Generate the three parallel arrays (aKWSince, aKWCFlag, aKWCFlagPolarity)
/// as C code, given the keyword index mapping from mkkeywordhash output.
fn generate_keyword_arrays(keyword_indices: &[(usize, String)], dialect: &str) -> String {
    let max_idx = keyword_indices.iter().map(|(i, _)| *i).max().unwrap_or(0);
    let array_len = max_idx + 1;

    let mut since = vec![0i32; array_len];
    let mut cflag = vec![0u32; array_len];
    let mut polarity = vec![0u8; array_len];

    let cflag_map = KeywordCflagMap::from_mkkeywordhash();

    for (idx, name) in keyword_indices {
        since[*idx] = keyword_since_version(name);
        if let Some((cflag_val, pol)) = cflag_map.get(name) {
            cflag[*idx] = cflag_val;
            polarity[*idx] = pol;
        }
    }

    let prefix = format!("synq_{}", dialect);

    let fmt_array = |values: &[String], c_type: &str, name: &str| -> String {
        let mut s = format!(
            "static const {} {}_{}[{}] = {{",
            c_type, prefix, name, array_len
        );
        for (i, v) in values.iter().enumerate() {
            if i % 13 == 0 {
                s.push_str("\n  ");
            }
            s.push_str(v);
            if i + 1 < values.len() {
                s.push(',');
            }
        }
        s.push_str("\n};\n");
        s
    };

    let since_strs: Vec<String> = since.iter().map(|v| v.to_string()).collect();
    let cflag_strs: Vec<String> = cflag
        .iter()
        .map(|v| {
            if *v == 0 {
                "0".to_string()
            } else {
                format!("{:#010x}", v)
            }
        })
        .collect();
    let pol_strs: Vec<String> = polarity.iter().map(|v| v.to_string()).collect();

    let mut out = String::new();
    out.push_str(&fmt_array(&since_strs, "int32_t", "aKWSince"));
    out.push_str(&fmt_array(&cflag_strs, "uint32_t", "aKWCFlag"));
    out.push_str(&fmt_array(&pol_strs, "uint8_t", "aKWCFlagPolarity"));
    out
}

/// The version+cflag check code that replaces `*pType = aKWCode[i]; break;`
/// in `synq_sqlite3_keywordCode`. Uses unprefixed `aKWCode` because the
/// CTransformer's `replace_all` will add the dialect prefix later.
fn keyword_check_code() -> &'static str {
    r#"/* Version check: skip keywords newer than target version. */
    if( synq_sqlite_aKWSince[i] != 0 && SYNQ_VER_LT(config, synq_sqlite_aKWSince[i]) ){
      break;
    }
    /* CFlag check with polarity. */
    if( synq_sqlite_aKWCFlag[i] != 0 ){
      int bit_set = SYNQ_HAS_CFLAG(config, synq_sqlite_aKWCFlag[i]) != 0;
      int is_enable = synq_sqlite_aKWCFlagPolarity[i];
      if( bit_set != is_enable ){
        break;
      }
    }
    *pType = aKWCode[i];
    break;"#
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    /// Parse sqlite_cflags.h and verify every entry in SYNQ_CFLAG_TABLE matches.
    #[test]
    fn synq_cflag_table_matches_header() {
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../syntaqlite-runtime/include/syntaqlite/sqlite_cflags.h"
        ));

        // Parse all "#define SYNQ_SQLITE_* 0x..." lines from the header.
        let mut header_defines: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();
        for line in header.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("#define SYNQ_SQLITE_") {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = format!("SYNQ_SQLITE_{}", parts[0]);
                    if let Some(hex) = parts[1].strip_prefix("0x") {
                        if let Ok(val) = u32::from_str_radix(hex, 16) {
                            header_defines.insert(name, val);
                        }
                    }
                }
            }
        }

        // Every entry in SYNQ_CFLAG_TABLE must match the header.
        for (name, value) in super::SYNQ_CFLAG_TABLE {
            let header_val = header_defines.get(*name);
            assert_eq!(
                header_val,
                Some(value),
                "SYNQ_CFLAG_TABLE entry {name}={value:#010x} does not match sqlite_cflags.h (got {:?})",
                header_val
            );
        }

        // Every entry in the header must be in SYNQ_CFLAG_TABLE.
        let table_names: std::collections::HashSet<&str> =
            super::SYNQ_CFLAG_TABLE.iter().map(|(n, _)| *n).collect();
        for (name, val) in &header_defines {
            assert!(
                table_names.contains(name.as_str()),
                "sqlite_cflags.h defines {name}={val:#010x} but it is missing from SYNQ_CFLAG_TABLE"
            );
        }
    }

    #[test]
    fn extract_terminals_collects_rhs_tokens_but_excludes_id() {
        let y = r#"
%token PERFETTO MACRO.
%fallback ID PERFETTO MODULE.
cmd ::= INCLUDE PERFETTO MODULE ID DOT ID.
cmd ::= CREATE PERFETTO MACRO ID LP RP AS ANY.
"#;
        let got: BTreeSet<String> = super::extract_terminals_from_y(&[y]).into_iter().collect();
        let want: BTreeSet<String> = [
            "ANY", "AS", "CREATE", "DOT", "INCLUDE", "LP", "MACRO", "MODULE", "PERFETTO", "RP",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        assert_eq!(got, want);
        assert!(!got.contains("ID"));
    }
}

pub(crate) fn generate_keyword_hash(
    extract_result: &TokenizerExtractResult,
    dialect: &str,
    extra_keywords: &[String],
) -> Result<String, String> {
    let mut cmd = crate::sqlite_util::self_subcommand("mkkeyword")?;

    let _kw_file;
    if !extra_keywords.is_empty() {
        let f = tempfile::NamedTempFile::new()
            .map_err(|e| format!("Failed to create keyword temp file: {e}"))?;
        fs::write(f.path(), extra_keywords.join("\n"))
            .map_err(|e| format!("Failed to write keyword file: {e}"))?;
        cmd.arg("--extra-file").arg(f.path());
        _kw_file = Some(f);
    } else {
        _kw_file = None;
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to spawn mkkeyword subprocess: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "mkkeyword failed with exit code: {}\n{}",
            output.status, stderr
        ));
    }

    let generated_code = String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8 in mkkeyword output: {e}"))?;

    // Parse keyword indices from testcase comments before any transformations.
    let keyword_indices = parse_keyword_indices(&generated_code);
    let keyword_arrays = generate_keyword_arrays(&keyword_indices, dialect);

    let kw_text_sym = format!("synq_{}_zKWText", dialect);
    let kw_offset_sym = format!("synq_{}_aKWOffset", dialect);
    let kw_len_sym = format!("synq_{}_aKWLen", dialect);
    let kw_code_sym = format!("synq_{}_aKWCode", dialect);
    let kw_count_sym = format!("synq_{}_nKeyword", dialect);

    let processed_code = crate::c_source::c_transformer::CTransformer::new(&generated_code)
        .remove_function("sqlite3KeywordCode")
        .remove_function("sqlite3_keyword_name")
        .remove_function("sqlite3_keyword_count")
        .remove_function("sqlite3_keyword_check")
        .remove_lines_matching("#define SQLITE_N_KEYWORD")
        .rename_function("keywordCode", "synq_sqlite3_keywordCode")
        .remove_static("synq_sqlite3_keywordCode")
        // Add config parameter to keywordCode signature.
        .replace_in_function(
            "synq_sqlite3_keywordCode",
            "synq_sqlite3_keywordCode(const char *z",
            "synq_sqlite3_keywordCode(const SyntaqliteDialectConfig *config, const char *z",
        )
        // Replace the simple assignment with version/cflag checks.
        .replace_in_function(
            "synq_sqlite3_keywordCode",
            "*pType = aKWCode[i];\n    break;",
            keyword_check_code(),
        )
        .remove_static("zKWText")
        .remove_static("aKWOffset")
        .remove_static("aKWLen")
        .remove_static("aKWCode")
        .replace_all("zKWText", &kw_text_sym)
        .replace_all("aKWOffset", &kw_offset_sym)
        .replace_all("aKWLen", &kw_len_sym)
        .replace_all("aKWCode", &kw_code_sym)
        .replace_all("TK_", "SYNTAQLITE_TK_")
        .finish();

    // Insert keyword arrays before the keywordCode function.
    let processed_code = {
        let fn_marker = "int synq_sqlite3_keywordCode(";
        match processed_code.find(fn_marker) {
            Some(pos) => {
                let mut result = processed_code[..pos].to_string();
                result.push_str(&keyword_arrays);
                result.push('\n');
                result.push_str(&processed_code[pos..]);
                result
            }
            None => processed_code,
        }
    };

    let mut w = CWriter::new();
    w.sqlite_file_header();
    w.include_local("syntaqlite_ext/sqlite_compat.h");
    w.include_local(&format!("syntaqlite_{dialect}/{dialect}_tokens.h"));
    w.include_local("syntaqlite/dialect_config.h");
    w.include_local("syntaqlite/sqlite_cflags.h");
    w.newline();

    w.fragment(&extract_result.upper_to_lower);
    w.newline();
    w.fragment(&extract_result.char_map);
    w.newline();

    w.fragment(&processed_code);
    w.newline();
    w.line(&format!(
        "const unsigned int {kw_count_sym} = sizeof({kw_code_sym}) / sizeof({kw_code_sym}[0]);"
    ));
    w.newline();

    Ok(w.finish())
}

pub(crate) fn generate_keyword_h() -> String {
    let mut w = CWriter::new();
    w.sqlite_file_header();
    w.header_guard_start("SYNTAQLITE_SQLITE_KEYWORD_H");
    w.newline();
    w.line("#include \"syntaqlite/dialect_config.h\"");
    w.newline();
    w.line("int synq_sqlite3_keywordCode(const SyntaqliteDialectConfig *config, const char* z, int n, int* pType);");
    w.newline();
    w.header_guard_end("SYNTAQLITE_SQLITE_KEYWORD_H");
    w.finish()
}

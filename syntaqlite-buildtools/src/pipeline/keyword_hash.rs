// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Stage 3 keyword hash generation: reads pre-extracted cflag data and calls
//! the mkkeyword subprocess to produce the final `keyword.c`.

use std::collections::HashMap;
use std::fs;

use crate::util::c_source::c_transformer::CTransformer;
use super::sqlite_fragments::SqliteFragments;
use super::writers::c_writer::CWriter;
use crate::TokenizerExtractResult;

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

/// Parse the pre-extracted keyword cflag data.
///
/// Format: one line per keyword, tab-separated:
///   KEYWORD_NAME<tab>cflag_value<tab>polarity
///
/// Lines starting with `#` are comments. Empty lines are skipped.
fn parse_keyword_cflags(data: &str) -> HashMap<String, (u32, u8)> {
    let mut map = HashMap::new();
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let name = parts[0].to_string();
            let cflag: u32 = parts[1].parse().unwrap_or(0);
            let polarity: u8 = parts[2].parse().unwrap_or(0);
            map.insert(name, (cflag, polarity));
        }
    }
    map
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

/// Generate the three parallel arrays (aKWSince, aKWCFlag, aKWCFlagPolarity)
/// as C code, given the keyword index mapping from mkkeywordhash output.
fn generate_keyword_arrays(
    keyword_indices: &[(usize, String)],
    cflag_map: &HashMap<String, (u32, u8)>,
    dialect: &str,
) -> String {
    let max_idx = keyword_indices.iter().map(|(i, _)| *i).max().unwrap_or(0);
    let array_len = max_idx + 1;

    let mut since = vec![0i32; array_len];
    let mut cflag = vec![0u32; array_len];
    let mut polarity = vec![0u8; array_len];

    for (idx, name) in keyword_indices {
        since[*idx] = keyword_since_version(name);
        if let Some(&(cflag_val, pol)) = cflag_map.get(name) {
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

/// Generate keyword hash lookup as a single `.c` file.
///
/// Uses pre-extracted cflag data from `fragments.keyword_cflags` instead of
/// parsing `mkkeywordhash.c` at codegen time.
pub fn generate(
    extract_result: &TokenizerExtractResult,
    fragments: &SqliteFragments,
    dialect: &str,
    extra_keywords: &[String],
) -> Result<String, String> {
    let cflag_map = parse_keyword_cflags(fragments.keyword_cflags);

    let mut cmd = crate::util::self_subcommand::self_subcommand("mkkeyword")?;

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
    let keyword_arrays = generate_keyword_arrays(&keyword_indices, &cflag_map, dialect);

    let kw_text_sym = format!("synq_{}_zKWText", dialect);
    let kw_offset_sym = format!("synq_{}_aKWOffset", dialect);
    let kw_len_sym = format!("synq_{}_aKWLen", dialect);
    let kw_code_sym = format!("synq_{}_aKWCode", dialect);
    let kw_count_sym = format!("synq_{}_nKeyword", dialect);

    let processed_code = CTransformer::new(&generated_code)
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

/// Generate the `sqlite_keyword.h` header.
pub fn generate_keyword_h() -> String {
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

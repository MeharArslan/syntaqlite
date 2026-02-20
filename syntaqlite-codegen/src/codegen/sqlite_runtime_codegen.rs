// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fs;

use syntaqlite_codegen_utils::{c_extractor, c_transformer};

use crate::TokenizerExtractResult;
use crate::util::naming;
use crate::util::subprocess;
use crate::writers::c_writer::CWriter;

pub(crate) fn extract_tokenizer(
    tokenize_c_path: &str,
    dialect: &str,
) -> Result<(String, TokenizerExtractResult), String> {
    let tokenize_content = fs::read_to_string(tokenize_c_path)
        .map_err(|e| format!("Failed to read {tokenize_c_path}: {e}"))?;
    let tokenize_extractor = c_extractor::CExtractor::new(&tokenize_content);

    let global_c = "third_party/src/sqlite/src/global.c";
    let global_content =
        fs::read_to_string(global_c).map_err(|e| format!("Failed to read {global_c}: {e}"))?;
    let global_extractor = c_extractor::CExtractor::new(&global_content);

    let sqliteint_h = "third_party/src/sqlite/src/sqliteInt.h";
    let sqliteint_content = fs::read_to_string(sqliteint_h)
        .map_err(|e| format!("Failed to read {sqliteint_h}: {e}"))?;
    let sqliteint_extractor = c_extractor::CExtractor::new(&sqliteint_content);

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

    let output = c_transformer::CTransformer::new(&combined)
        .add_array_static("sqlite3CtypeMap")
        .replace_in_function("sqlite3GetToken", "keywordCode", "synq_sqlite3_keywordCode")
        .rename_function(
            "sqlite3GetToken",
            &format!("Synq{}GetToken", naming::pascal_case(dialect)),
        )
        .replace_all("TK_", "SYNTAQLITE_TK_")
        .finish();

    Ok((
        output,
        TokenizerExtractResult {
            char_map: char_map.text,
            upper_to_lower: upper_to_lower.text,
        },
    ))
}

/// Extract terminal symbols (potential keywords) from extension `.y` grammar files.
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
                if is_keyword_like(sym.name) {
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

pub(crate) fn generate_keyword_hash(
    extract_result: &TokenizerExtractResult,
    dialect: &str,
    extra_keywords: &[String],
) -> Result<String, String> {
    let mut cmd = subprocess::self_subcommand("mkkeyword")?;

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

    let processed_code = c_transformer::CTransformer::new(&generated_code)
        .remove_function("sqlite3KeywordCode")
        .remove_function("sqlite3_keyword_name")
        .remove_function("sqlite3_keyword_count")
        .remove_function("sqlite3_keyword_check")
        .remove_lines_matching("#define SQLITE_N_KEYWORD")
        .rename_function("keywordCode", "synq_sqlite3_keywordCode")
        .remove_static("synq_sqlite3_keywordCode")
        .replace_all("TK_", "SYNTAQLITE_TK_")
        .finish();

    let mut w = CWriter::new();
    w.sqlite_file_header();
    w.include_local("syntaqlite_ext/sqlite_compat.h");
    w.include_local(&format!("syntaqlite_{dialect}/{dialect}_tokens.h"));
    w.newline();

    w.fragment(&extract_result.upper_to_lower);
    w.newline();
    w.fragment(&extract_result.char_map);
    w.newline();

    w.fragment(&processed_code);
    w.newline();

    Ok(w.finish())
}

pub(crate) fn generate_keyword_h() -> String {
    let mut w = CWriter::new();
    w.sqlite_file_header();
    w.header_guard_start("SYNTAQLITE_SQLITE_KEYWORD_H");
    w.newline();
    w.line("int synq_sqlite3_keywordCode(const char* z, int n, int* pType);");
    w.newline();
    w.header_guard_end("SYNTAQLITE_SQLITE_KEYWORD_H");
    w.finish()
}

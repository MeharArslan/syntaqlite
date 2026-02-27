// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Stage 3 tokenizer assembly: combines pre-extracted C fragments with
//! dialect-specific naming to produce the final `tokenize.c`.

use crate::util::c_source::c_transformer::CTransformer;
use super::sqlite_fragments::SqliteFragments;
use crate::util::pascal_case;
use super::writers::c_writer::CWriter;
use crate::TokenizerExtractResult;

/// Assemble the tokenizer C source from pre-extracted fragments.
///
/// This replaces the old `extract_tokenizer()` path: instead of running
/// `CExtractor` on raw SQLite source at codegen time, we read committed
/// fragment files and apply only the dialect-specific `CTransformer` pass.
pub fn assemble(
    fragments: &SqliteFragments,
    dialect: &str,
) -> Result<(String, TokenizerExtractResult), String> {
    let combined = {
        let mut w = CWriter::new();
        w.sqlite_file_header();
        w.include_local("syntaqlite_ext/sqlite_compat.h");
        w.include_local(&format!("syntaqlite_{dialect}/{dialect}_tokens.h"));
        w.include_local("csrc/sqlite_keyword.h")
            .newline()
            .fragment(&fragments.cc_defines)
            .newline()
            .fragment(&fragments.ctype_map)
            .newline()
            .fragment(&fragments.is_macros)
            .newline()
            .fragment(&fragments.id_char)
            .newline()
            .fragment(&fragments.ai_class)
            .newline()
            .fragment(&fragments.get_token_fn);
        w.finish()
    };

    let get_token_name = format!("Synq{}GetToken", pascal_case(dialect));
    let get_token_base = format!("{}_base", get_token_name);
    let output = CTransformer::new(&combined)
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
            char_map: fragments.char_map.to_string(),
            upper_to_lower: fragments.upper_to_lower.to_string(),
        },
    ))
}

/// Generate the public `GetToken` wrapper that calls the `_base` function then
/// applies version-dependent token reclassification (the "postlude").
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

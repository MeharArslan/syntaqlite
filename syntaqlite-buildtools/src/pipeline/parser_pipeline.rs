// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fs;
use std::path::Path;

use crate::util::c_source::c_transformer::CTransformer;

// Embed lempar.c template (needed by the library)
const LEMPAR_C: &[u8] = include_bytes!("../../sqlite-vendored/sources/lempar.c");

pub(crate) fn generate_parser(
    actions_dir: &str,
    parser_name: &str,
    output_dir: &str,
) -> Result<(), String> {
    let body = concatenate_y_files(actions_dir)?;
    let directive = format!("%name {parser_name}\n");
    let mut grammar_bytes = directive.into_bytes();
    grammar_bytes.extend_from_slice(&body);
    generate_parser_with_grammar_bytes(&grammar_bytes, parser_name, output_dir)
}

/// Concatenate in-memory .y file contents (already sorted by caller).
pub(crate) fn concatenate_y_contents(files: &[(String, String)]) -> Result<Vec<u8>, String> {
    if files.is_empty() {
        return Err("no .y files provided".to_string());
    }
    let mut combined = Vec::new();
    for (_name, content) in files {
        combined.extend_from_slice(content.as_bytes());
        combined.push(b'\n');
    }
    Ok(combined)
}

/// Generate parser from in-memory .y file contents (merged base + extensions).
///
/// `parser_name` is the Lemon `%name` directive value (e.g. `"SynqSqliteParse"`),
/// prepended to the concatenated grammar so each dialect gets its own symbol prefix.
pub(crate) fn generate_parser_from_contents(
    y_files: &[(String, String)],
    parser_name: &str,
    output_dir: &str,
) -> Result<(), String> {
    let body = concatenate_y_contents(y_files)?;
    let directive = format!("%name {parser_name}\n");
    let mut grammar_bytes = directive.into_bytes();
    grammar_bytes.extend_from_slice(&body);
    generate_parser_with_grammar_bytes(&grammar_bytes, parser_name, output_dir)
}

pub(crate) fn generate_parser_with_grammar_bytes(
    grammar_bytes: &[u8],
    parser_name: &str,
    output_dir: &str,
) -> Result<(), String> {
    let work_dir = Path::new(output_dir);
    fs::create_dir_all(work_dir).map_err(|e| format!("Failed to create output directory: {e}"))?;

    let parse_y_path = work_dir.join("parse.y");
    fs::write(&parse_y_path, grammar_bytes).map_err(|e| format!("Failed to write parse.y: {e}"))?;

    let extracted_grammar_path = work_dir.join("parse_extracted.h");
    let parse_y_str = parse_y_path
        .to_str()
        .ok_or_else(|| "Invalid parse.y path".to_string())?;
    let extracted_grammar_str = extracted_grammar_path
        .to_str()
        .ok_or_else(|| "Invalid extracted grammar path".to_string())?;

    super::grammar_codegen::extract_grammar(parse_y_str, Some(extracted_grammar_str))?;

    let lempar_path = work_dir.join("lempar.c");
    fs::write(&lempar_path, LEMPAR_C).map_err(|e| format!("Failed to write lempar.c: {e}"))?;
    let lempar_str = lempar_path
        .to_str()
        .ok_or_else(|| "Invalid lempar.c path".to_string())?;
    let template_arg = format!("-T{lempar_str}");

    let status = run_lemon(&template_arg, parse_y_str)?;

    if !status.success() {
        return Err(format!("Lemon failed with exit code: {status}"));
    }

    let parse_c = work_dir.join("parse.c");
    let parse_h = work_dir.join("parse.h");

    if !parse_c.exists() {
        return Err("Lemon did not generate parse.c".to_string());
    }
    if !parse_h.exists() {
        return Err("Lemon did not generate parse.h".to_string());
    }

    patch_generated_parser_files(parser_name, &parse_c, &parse_h)?;

    Ok(())
}

fn run_lemon(template_arg: &str, parse_y_str: &str) -> Result<std::process::ExitStatus, String> {
    crate::util::self_subcommand::self_subcommand("lemon")?
        .arg("-l")
        .arg(template_arg)
        .arg(parse_y_str)
        .status()
        .map_err(|e| format!("Failed to spawn lemon subprocess: {e}"))
}

fn patch_generated_parser_files(
    parser_name: &str,
    parse_c: &Path,
    parse_h: &Path,
) -> Result<(), String> {
    let parse_c_content = fs::read_to_string(parse_c)
        .map_err(|e| format!("Failed to read {}: {e}", parse_c.display()))?;
    let parse_h_content = fs::read_to_string(parse_h)
        .map_err(|e| format!("Failed to read {}: {e}", parse_h.display()))?;

    let parse_c_patched = CTransformer::new(&parse_c_content)
        .append(&expected_tokens_c_snippet(parser_name))
        .finish();
    let parse_h_patched = CTransformer::new(&parse_h_content)
        .append(&expected_tokens_h_snippet(parser_name))
        .finish();

    fs::write(parse_c, parse_c_patched)
        .map_err(|e| format!("Failed to write {}: {e}", parse_c.display()))?;
    fs::write(parse_h, parse_h_patched)
        .map_err(|e| format!("Failed to write {}: {e}", parse_h.display()))?;

    Ok(())
}

fn expected_tokens_h_snippet(parser_name: &str) -> String {
    format!(
        "\n/* syntaqlite extension: expected terminals for current parser state. */\n\
int {parser_name}ExpectedTokens(void* parser, int* out_tokens, int out_cap);\n"
    )
}

fn expected_tokens_c_snippet(parser_name: &str) -> String {
    format!(
        "\n\
/* syntaqlite extension: enumerate terminals that can be shifted/reduced from\n\
** the parser's current state. Returns the total number of expected tokens,\n\
** even when out_tokens/out_cap only request a prefix. */\n\
static YYACTIONTYPE synq_find_reduce_action_safe(YYACTIONTYPE stateno, YYCODETYPE iLookAhead) {{\n\
  int i;\n\
  if( stateno>YY_REDUCE_COUNT ) return yy_default[stateno];\n\
  i = yy_reduce_ofst[stateno] + iLookAhead;\n\
  if( i<0 || i>=YY_ACTTAB_COUNT || yy_lookahead[i]!=iLookAhead ) {{\n\
    return yy_default[stateno];\n\
  }}\n\
  return yy_action[i];\n\
}}\n\
\n\
/* Like yy_find_shift_action but skips YYWILDCARD and YYFALLBACK paths.\n\
** Wildcard matches are for error recovery (ANY token) and fallback matches\n\
** accept keywords as identifiers — neither should appear as keyword\n\
** autocompletion suggestions. */\n\
static YYACTIONTYPE synq_find_shift_action_strict(\n\
  YYCODETYPE iLookAhead,\n\
  YYACTIONTYPE stateno\n\
){{\n\
  int i;\n\
  if( stateno>YY_MAX_SHIFT ) return stateno;\n\
  i = yy_shift_ofst[stateno];\n\
  assert( i>=0 );\n\
  assert( i+YYNTOKEN<=(int)YY_NLOOKAHEAD );\n\
  i += iLookAhead;\n\
  if( yy_lookahead[i]!=iLookAhead ){{\n\
    /* No specific entry — skip fallback and wildcard, use default. */\n\
    return yy_default[stateno];\n\
  }}\n\
  return yy_action[i];\n\
}}\n\
\n\
static int synq_can_lookahead(yyParser* p, int token) {{\n\
  YYACTIONTYPE stack_states[YYSTACKDEPTH + 1];\n\
  int top = 0;\n\
  int i = 0;\n\
  int steps = 0;\n\
\n\
  if( p==0 || p->yytos==0 ) return 0;\n\
\n\
  top = (int)(p->yytos - p->yystack);\n\
  if( top<0 || top>YYSTACKDEPTH ) return 0;\n\
  for(i=0; i<=top; i++) {{\n\
    stack_states[i] = p->yystack[i].stateno;\n\
  }}\n\
\n\
  while( steps++ < 10000 ) {{\n\
    YYACTIONTYPE action = synq_find_shift_action_strict((YYCODETYPE)token, stack_states[top]);\n\
\n\
    if( action==YY_ERROR_ACTION || action==YY_NO_ACTION ) return 0;\n\
    if( action==YY_ACCEPT_ACTION ) return token==0;\n\
    if( action<=YY_MAX_SHIFT ) return 1;\n\
\n\
    /* Shift-reduce: the token is consumed (shifted) then a reduce follows.\n\
    ** This means the token IS accepted, same as a pure shift. */\n\
    if( action>=YY_MIN_SHIFTREDUCE && action<=YY_MAX_SHIFTREDUCE ) return 1;\n\
\n\
    if( action>=YY_MIN_REDUCE && action<=YY_MAX_REDUCE ) {{\n\
      int rule = (int)(action - YY_MIN_REDUCE);\n\
      int yysize = yyRuleInfoNRhs[rule];\n\
      YYACTIONTYPE goto_state;\n\
\n\
      top += yysize;  /* yyRuleInfoNRhs is negative rhs-size */\n\
      if( top<0 ) return 0;\n\
\n\
      goto_state = synq_find_reduce_action_safe(stack_states[top], yyRuleInfoLhs[rule]);\n\
      if( goto_state==YY_ERROR_ACTION || goto_state==YY_NO_ACTION ) return 0;\n\
\n\
      if( top>=YYSTACKDEPTH ) return 0;\n\
      top++;\n\
      stack_states[top] = goto_state;\n\
      continue;\n\
    }}\n\
\n\
    return 0;\n\
  }}\n\
\n\
  return 0;\n\
}}\n\
\n\
int {parser_name}ExpectedTokens(void* parser, int* out_tokens, int out_cap) {{\n\
  int n = 0;\n\
  int token = 0;\n\
  yyParser* p = (yyParser*)parser;\n\
\n\
  if( p==0 || p->yytos==0 ) return 0;\n\
\n\
  for(token=1; token<YYNTOKEN; token++) {{\n\
    if( !synq_can_lookahead(p, token) ) continue;\n\
    if( out_tokens && n<out_cap ) out_tokens[n] = token;\n\
    n++;\n\
  }}\n\
\n\
  return n;\n\
}}\n"
    )
}

/// Read all .y files from a directory, sort by name, and concatenate their contents.
fn concatenate_y_files(dir: &str) -> Result<Vec<u8>, String> {
    let y_files = crate::read_named_files_from_dir(dir, "y")?;
    if y_files.is_empty() {
        return Err(format!("No .y files found in {dir}"));
    }
    concatenate_y_contents(&y_files)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::patch_generated_parser_files;

    #[test]
    fn patch_generated_parser_files_appends_expected_tokens_helpers() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let parse_c = dir.path().join("parse.c");
        let parse_h = dir.path().join("parse.h");

        fs::write(&parse_c, "/* parse.c */\n").expect("write parse.c");
        fs::write(&parse_h, "/* parse.h */\n").expect("write parse.h");

        patch_generated_parser_files("SynqFooParse", &parse_c, &parse_h)
            .expect("patch parser files");

        let parse_c_out = fs::read_to_string(&parse_c).expect("read parse.c");
        let parse_h_out = fs::read_to_string(&parse_h).expect("read parse.h");

        assert!(parse_c_out.contains("int SynqFooParseExpectedTokens("));
        assert!(parse_c_out.contains("yy_find_shift_action"));
        assert!(parse_h_out.contains("int SynqFooParseExpectedTokens("));
    }
}

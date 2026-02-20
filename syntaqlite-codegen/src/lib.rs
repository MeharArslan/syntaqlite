// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

pub mod amalgamate;
pub mod ast_codegen;
pub mod base_files;
pub mod c_writer;
pub mod fmt_compiler;
pub mod grammar_parser;
pub mod lemon;
pub mod mkkeyword;
pub mod node_parser;
mod run;
pub mod rust_writer;

use std::fs;
use std::path::Path;
use syntaqlite_codegen_utils::{c_extractor, c_transformer};

pub struct TokenizerExtractResult {
    pub char_map: String,
    pub upper_to_lower: String,
}

/// Parse `#define SYNTAQLITE_TK_NAME VALUE` lines from lemon's parse.h output.
/// Returns structured `(name, value)` pairs where name is the short name (e.g. "ABORT").
pub fn extract_token_defines(parse_h: &str) -> Vec<(String, u32)> {
    let mut tokens = Vec::new();
    for line in parse_h.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("#define SYNTAQLITE_TK_") {
            let mut parts = rest.split_whitespace();
            if let (Some(name), Some(value_str)) = (parts.next(), parts.next()) {
                if let Ok(value) = value_str.parse::<u32>() {
                    tokens.push((name.to_string(), value));
                }
            }
        }
    }
    tokens
}

// Embed lempar.c template (needed by the library)
const LEMPAR_C: &[u8] = include_bytes!("../sqlite/lempar.c");

pub fn extract_grammar(input_path: &str, output_path: Option<&str>) -> Result<(), String> {
    // Read input file
    let input_text = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read {}: {}", input_path, e))?;

    // Parse grammar
    let grammar = grammar_parser::LemonGrammar::parse(&input_text)
        .map_err(|e| format!("Parse error at {}:{}: {}", e.line, e.column, e.message))?;

    // Generate C header
    let c_code = generate_header(&grammar)?;

    // Write output
    if let Some(output) = output_path {
        fs::write(output, c_code).map_err(|e| format!("Failed to write {}: {}", output, e))?;
    } else {
        print!("{}", c_code);
    }

    Ok(())
}

fn generate_header(grammar: &grammar_parser::LemonGrammar) -> Result<String, String> {
    let mut w = c_writer::CWriter::new();

    // File header (SQLite-derived)
    w.sqlite_file_header();

    // Header guard
    let guard = "GRAMMAR_TOKENS_H";
    w.header_guard_start(guard);

    // Includes
    w.include_system("stdint.h");
    w.newline();

    w.extern_c_start();

    // Token enum
    w.section("Token Types");
    let mut token_variants: Vec<(&str, Option<i32>)> = vec![("TOKEN_INVALID", Some(0))];
    for (i, token) in grammar.tokens.iter().enumerate() {
        token_variants.push((token.name, Some((i + 1) as i32)));
    }
    w.typedef_enum("TokenType", &token_variants);
    w.newline();

    // Token count constant
    w.comment(&format!("Total number of tokens: {}", grammar.tokens.len()));
    w.line(&format!("#define TOKEN_COUNT {}", grammar.tokens.len() + 1));
    w.newline();

    w.extern_c_end();
    w.newline();

    w.header_guard_end(guard);

    Ok(w.finish())
}

pub fn extract_tokenizer(
    tokenize_c_path: &str,
    dialect: &str,
) -> Result<(String, TokenizerExtractResult), String> {
    let tokenize_content = fs::read_to_string(tokenize_c_path)
        .map_err(|e| format!("Failed to read {}: {}", tokenize_c_path, e))?;
    let tokenize_extractor = c_extractor::CExtractor::new(&tokenize_content);

    let global_c = "third_party/src/sqlite/src/global.c";
    let global_content =
        fs::read_to_string(global_c).map_err(|e| format!("Failed to read {}: {}", global_c, e))?;
    let global_extractor = c_extractor::CExtractor::new(&global_content);

    let sqliteint_h = "third_party/src/sqlite/src/sqliteInt.h";
    let sqliteint_content = fs::read_to_string(sqliteint_h)
        .map_err(|e| format!("Failed to read {}: {}", sqliteint_h, e))?;
    let sqliteint_extractor = c_extractor::CExtractor::new(&sqliteint_content);

    // Extract pieces
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

    // Combine extracted pieces
    let combined = {
        let mut w = c_writer::CWriter::new();
        w.sqlite_file_header();
        w.include_local("syntaqlite_ext/sqlite_compat.h");
        w.include_local(&format!(
            "syntaqlite_{dialect}/{dialect}_tokens.h"
        ));
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

    // Transform the combined result
    let output = c_transformer::CTransformer::new(&combined)
        .add_array_static("sqlite3CtypeMap")
        .replace_in_function("sqlite3GetToken", "keywordCode", "synq_sqlite3_keywordCode")
        .rename_function("sqlite3GetToken", &format!("Synq{}GetToken", ast_codegen::pascal_case(dialect)))
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

pub fn generate_parser(actions_dir: &str, output_dir: &str) -> Result<(), String> {
    let grammar_bytes = concatenate_y_files(actions_dir)?;
    let work_dir = Path::new(output_dir);
    fs::create_dir_all(work_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    // Step 1: Write raw grammar to working directory
    let raw_parse_y_path = work_dir.join("parse_raw.y");
    fs::write(&raw_parse_y_path, &grammar_bytes)
        .map_err(|e| format!("Failed to write parse_raw.y: {}", e))?;

    // Step 2: Extract grammar using extract_grammar
    let extracted_grammar_path = work_dir.join("parse_extracted.h");
    let raw_parse_y_str = raw_parse_y_path
        .to_str()
        .ok_or_else(|| "Invalid raw parse.y path".to_string())?;
    let extracted_grammar_str = extracted_grammar_path
        .to_str()
        .ok_or_else(|| "Invalid extracted grammar path".to_string())?;

    extract_grammar(raw_parse_y_str, Some(extracted_grammar_str))?;

    // Step 3: Write lempar.c template to working directory
    let lempar_path = work_dir.join("lempar.c");
    fs::write(&lempar_path, LEMPAR_C).map_err(|e| format!("Failed to write lempar.c: {}", e))?;

    // Step 4: Write the original grammar for lemon processing
    // (lemon needs the full .y file with rules, not just the extracted tokens)
    let parse_y_path = work_dir.join("parse.y");
    fs::write(&parse_y_path, &grammar_bytes)
        .map_err(|e| format!("Failed to write parse.y: {}", e))?;

    // Step 5: Run lemon with -T option pointing to our template
    // Spawn ourselves as a subprocess with the lemon subcommand
    // Note: lemon expects -T and the path combined as a single argument: -T/path/to/template.c
    let parse_y_str = parse_y_path
        .to_str()
        .ok_or_else(|| "Invalid parse.y path".to_string())?;
    let lempar_str = lempar_path
        .to_str()
        .ok_or_else(|| "Invalid lempar.c path".to_string())?;
    let template_arg = format!("-T{}", lempar_str);

    let status = std::process::Command::new(
        std::env::current_exe().map_err(|e| format!("Failed to get current executable: {}", e))?,
    )
    .arg("lemon")
    .arg("-l")
    .arg(&template_arg)
    .arg(parse_y_str)
    .status()
    .map_err(|e| format!("Failed to spawn lemon subprocess: {}", e))?;

    if !status.success() {
        return Err(format!("Lemon failed with exit code: {}", status));
    }

    // Verify outputs were generated
    let parse_c = work_dir.join("parse.c");
    let parse_h = work_dir.join("parse.h");

    if !parse_c.exists() {
        return Err("Lemon did not generate parse.c".to_string());
    }
    if !parse_h.exists() {
        return Err("Lemon did not generate parse.h".to_string());
    }

    Ok(())
}

/// Extract terminal symbols (potential keywords) from extension `.y` grammar
/// files using the Lemon grammar parser.
///
/// Collects uppercase identifiers from `%token` declarations, `%fallback`
/// directives, and RHS symbols in grammar rules. Duplicates against the base
/// keyword table are handled by `run_mkkeyword` (silently skipped).
pub fn extract_terminals_from_y(extension_y_contents: &[&str]) -> Vec<String> {
    use std::collections::HashSet;

    let mut terminals: HashSet<String> = HashSet::new();

    for content in extension_y_contents {
        let grammar = match grammar_parser::LemonGrammar::parse(content) {
            Ok(g) => g,
            Err(_) => continue,
        };

        // %token declarations
        for tok in &grammar.tokens {
            if is_keyword_like(tok.name) {
                terminals.insert(tok.name.to_string());
            }
        }

        // %fallback declarations (both target and token lists)
        for fb in &grammar.fallbacks {
            for tok in &fb.tokens {
                if is_keyword_like(tok) {
                    terminals.insert(tok.to_string());
                }
            }
        }

        // Uppercase RHS symbols in rules (terminals in Lemon are uppercase)
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

/// Returns true if `name` looks like a keyword token: purely uppercase
/// ASCII letters, at least 2 characters.
fn is_keyword_like(name: &str) -> bool {
    name.len() >= 2 && name.chars().all(|c| c.is_ascii_uppercase())
}

/// Generate keyword hash lookup as a single `.c` file.
///
/// Returns the content of `sqlite_keyword.c` containing tables, char maps,
/// and the `synq_sqlite3_keywordCode` function.
///
/// When `extra_keywords` is non-empty, those keywords are added to the
/// hash table alongside the base 148 SQLite keywords (duplicates are
/// silently skipped). This produces a fully self-contained keyword lookup
/// that recognises both base and extension keywords.
pub fn generate_keyword_hash(
    extract_result: &TokenizerExtractResult,
    dialect: &str,
    extra_keywords: &[String],
) -> Result<String, String> {
    let exe =
        std::env::current_exe().map_err(|e| format!("Failed to get current executable: {}", e))?;
    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("mkkeyword");

    // Write extra keywords to a temp file if any.
    let _kw_file; // keep alive
    if !extra_keywords.is_empty() {
        let f = tempfile::NamedTempFile::new()
            .map_err(|e| format!("Failed to create keyword temp file: {}", e))?;
        fs::write(f.path(), extra_keywords.join("\n"))
            .map_err(|e| format!("Failed to write keyword file: {}", e))?;
        cmd.arg("--extra-file").arg(f.path());
        _kw_file = Some(f);
    } else {
        _kw_file = None;
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to spawn mkkeyword subprocess: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "mkkeyword failed with exit code: {}\n{}",
            output.status, stderr
        ));
    }

    // Convert output to string for processing
    let generated_code = String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8 in mkkeyword output: {}", e))?;

    // Remove unwanted SQLite-specific wrapper functions using CTransformer
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

    let mut w = c_writer::CWriter::new();
    w.sqlite_file_header();
    w.include_local("syntaqlite_ext/sqlite_compat.h");
    w.include_local(&format!("syntaqlite_{dialect}/{dialect}_tokens.h"));
    w.newline();

    // Add char mapping arrays (upper_to_lower and char_map)
    w.fragment(&extract_result.upper_to_lower);
    w.newline();
    w.fragment(&extract_result.char_map);
    w.newline();

    // Add the mkkeyword-generated tables and function
    w.fragment(&processed_code);
    w.newline();

    Ok(w.finish())
}

/// Generate the `sqlite_keyword.h` header with the forward declaration for the keyword lookup function.
pub fn generate_keyword_h() -> String {
    let mut w = c_writer::CWriter::new();
    w.sqlite_file_header();
    w.header_guard_start("SYNTAQLITE_SQLITE_KEYWORD_H");
    w.newline();
    w.line("int synq_sqlite3_keywordCode(const char* z, int n, int* pType);");
    w.newline();
    w.header_guard_end("SYNTAQLITE_SQLITE_KEYWORD_H");
    w.finish()
}

/// Concatenate in-memory .y file contents (already sorted by caller).
pub fn concatenate_y_contents(files: &[(String, String)]) -> Result<Vec<u8>, String> {
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
/// Same pipeline as [`generate_parser`] but reads from memory instead of disk.
pub fn generate_parser_from_contents(
    y_files: &[(String, String)],
    output_dir: &str,
) -> Result<(), String> {
    let grammar_bytes = concatenate_y_contents(y_files)?;
    let work_dir = Path::new(output_dir);
    fs::create_dir_all(work_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let raw_parse_y_path = work_dir.join("parse_raw.y");
    fs::write(&raw_parse_y_path, &grammar_bytes)
        .map_err(|e| format!("Failed to write parse_raw.y: {}", e))?;

    let extracted_grammar_path = work_dir.join("parse_extracted.h");
    let raw_parse_y_str = raw_parse_y_path
        .to_str()
        .ok_or_else(|| "Invalid raw parse.y path".to_string())?;
    let extracted_grammar_str = extracted_grammar_path
        .to_str()
        .ok_or_else(|| "Invalid extracted grammar path".to_string())?;

    extract_grammar(raw_parse_y_str, Some(extracted_grammar_str))?;

    let lempar_path = work_dir.join("lempar.c");
    fs::write(&lempar_path, LEMPAR_C).map_err(|e| format!("Failed to write lempar.c: {}", e))?;

    let parse_y_path = work_dir.join("parse.y");
    fs::write(&parse_y_path, &grammar_bytes)
        .map_err(|e| format!("Failed to write parse.y: {}", e))?;

    let parse_y_str = parse_y_path
        .to_str()
        .ok_or_else(|| "Invalid parse.y path".to_string())?;
    let lempar_str = lempar_path
        .to_str()
        .ok_or_else(|| "Invalid lempar.c path".to_string())?;
    let template_arg = format!("-T{}", lempar_str);

    let status = std::process::Command::new(
        std::env::current_exe().map_err(|e| format!("Failed to get current executable: {}", e))?,
    )
    .arg("lemon")
    .arg("-l")
    .arg(&template_arg)
    .arg(parse_y_str)
    .status()
    .map_err(|e| format!("Failed to spawn lemon subprocess: {}", e))?;

    if !status.success() {
        return Err(format!("Lemon failed with exit code: {}", status));
    }

    let parse_c = work_dir.join("parse.c");
    let parse_h = work_dir.join("parse.h");

    if !parse_c.exists() {
        return Err("Lemon did not generate parse.c".to_string());
    }
    if !parse_h.exists() {
        return Err("Lemon did not generate parse.h".to_string());
    }

    Ok(())
}

/// Read all .y files from a directory, sort by name, and concatenate their contents.
fn concatenate_y_files(dir: &str) -> Result<Vec<u8>, String> {
    let dir_path = Path::new(dir);
    if !dir_path.is_dir() {
        return Err(format!("{} is not a directory", dir));
    }

    let mut y_files: Vec<_> = fs::read_dir(dir_path)
        .map_err(|e| format!("Failed to read directory {}: {}", dir, e))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("y") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    y_files.sort();

    if y_files.is_empty() {
        return Err(format!("No .y files found in {}", dir));
    }

    let mut combined = Vec::new();
    for path in &y_files {
        let content =
            fs::read(path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        combined.extend_from_slice(&content);
        combined.push(b'\n');
    }

    Ok(combined)
}

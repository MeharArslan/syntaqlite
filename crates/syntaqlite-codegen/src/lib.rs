pub mod grammar_parser;
pub mod lemon;

use std::fs;
use syntaqlite_codegen_utils::{c_extractor, c_transformer, c_writer};

pub fn extract_grammar(input_path: &str, output_path: Option<&str>) -> Result<(), String> {
    // Read input file
    let input_text = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read {}: {}", input_path, e))?;

    // Parse grammar
    let grammar = grammar_parser::LemonGrammar::parse(&input_text)
        .map_err(|e| format!("Parse error at {}:{}: {}", e.line, e.column, e.message))?;

    // Generate C header
    let c_code = generate_header(&grammar, input_path)?;

    // Write output
    if let Some(output) = output_path {
        fs::write(output, c_code).map_err(|e| format!("Failed to write {}: {}", output, e))?;
    } else {
        print!("{}", c_code);
    }

    Ok(())
}

fn generate_header(grammar: &grammar_parser::LemonGrammar, source: &str) -> Result<String, String> {
    let mut w = c_writer::CWriter::new();

    // File header
    w.file_header(source, "syntaqlite-codegen");

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

pub fn extract_tokenizer(tokenize_c_path: &str, output_path: &str) -> Result<(), String> {
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
    let is_macros = sqliteint_extractor.extract_specific_defines(&[
        "sqlite3Isspace",
        "sqlite3Isdigit",
        "sqlite3Isxdigit",
    ])?;
    let function = tokenize_extractor.extract_function("sqlite3GetToken")?;

    // Combine extracted pieces
    let combined = {
        let mut w = c_writer::CWriter::new();
        w.include_local("src/common/sqlite_compat.h")
            .newline()
            .fragment(&cc_defines)
            .newline()
            .fragment(&ctype_map)
            .newline()
            .fragment(&is_macros)
            .newline()
            .fragment(&ai_class)
            .newline()
            .fragment(&function);
        w.finish()
    };

    // Transform the combined result
    let output = c_transformer::CTransformer::new(&combined)
        .add_array_static("sqlite3CtypeMap")
        .rename_function("sqlite3GetToken", "synq_sqlite3GetToken")
        .finish();

    fs::write(output_path, output)
        .map_err(|e| format!("Failed to write {}: {}", output_path, e))?;

    Ok(())
}

pub mod grammar_parser;
pub mod lemon;

use std::fs;
use std::path::{Path, PathBuf};
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

/// Generate parser by processing SQLite grammar and running lemon
///
/// This function:
/// 1. Writes the provided grammar bytes to a temporary directory
/// 2. Extracts grammar tokens/rules using extract_grammar
/// 3. Writes the lempar.c template
/// 4. Runs lemon via subprocess to generate parse.c and parse.h
/// 5. Returns the path to the directory containing the outputs
///
/// # Arguments
/// * `grammar_bytes` - The parse.y grammar file content
/// * `template_bytes` - The lempar.c template file content
/// * `output_dir` - Optional output directory (uses tempdir if not specified)
///
/// # Returns
/// Path to the directory containing parse.c and parse.h
///
/// # Errors
/// Returns error if file operations or lemon execution fails
pub fn generate_parser(
    grammar_bytes: &[u8],
    template_bytes: &[u8],
    output_dir: Option<&str>,
) -> Result<PathBuf, String> {
    // Create or use provided output directory
    let temp_dir: Option<tempfile::TempDir>;
    let work_dir = if let Some(dir) = output_dir {
        let path = Path::new(dir);
        fs::create_dir_all(path)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
        path
    } else {
        temp_dir = Some(tempfile::TempDir::new()
            .map_err(|e| format!("Failed to create temp directory: {}", e))?);
        temp_dir.as_ref().unwrap().path()
    };

    // Step 1: Write raw grammar to working directory
    let raw_parse_y_path = work_dir.join("parse_raw.y");
    fs::write(&raw_parse_y_path, grammar_bytes)
        .map_err(|e| format!("Failed to write parse_raw.y: {}", e))?;

    // Step 2: Extract grammar using extract_grammar
    let extracted_grammar_path = work_dir.join("parse_extracted.h");
    let raw_parse_y_str = raw_parse_y_path.to_str()
        .ok_or_else(|| "Invalid raw parse.y path".to_string())?;
    let extracted_grammar_str = extracted_grammar_path.to_str()
        .ok_or_else(|| "Invalid extracted grammar path".to_string())?;

    extract_grammar(raw_parse_y_str, Some(extracted_grammar_str))?;

    // Step 3: Write lempar.c template to working directory
    let lempar_path = work_dir.join("lempar.c");
    fs::write(&lempar_path, template_bytes)
        .map_err(|e| format!("Failed to write lempar.c: {}", e))?;

    // Step 4: Write the original grammar for lemon processing
    // (lemon needs the full .y file with rules, not just the extracted tokens)
    let parse_y_path = work_dir.join("parse.y");
    fs::write(&parse_y_path, grammar_bytes)
        .map_err(|e| format!("Failed to write parse.y: {}", e))?;

    // Step 5: Run lemon with -T option pointing to our template
    // Spawn ourselves as a subprocess with the lemon subcommand
    let parse_y_str = parse_y_path.to_str()
        .ok_or_else(|| "Invalid parse.y path".to_string())?;
    let lempar_str = lempar_path.to_str()
        .ok_or_else(|| "Invalid lempar.c path".to_string())?;

    let status = std::process::Command::new(std::env::current_exe()
            .map_err(|e| format!("Failed to get current executable: {}", e))?)
        .arg("lemon")
        .arg("-T")
        .arg(lempar_str)
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

    Ok(work_dir.to_path_buf())
}

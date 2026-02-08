pub mod grammar_parser;
pub mod lemon;
pub mod mkkeyword;
mod run;

use std::fs;
use std::path::Path;
use syntaqlite_codegen_utils::{c_extractor, c_transformer, c_writer};

pub struct TokenizerExtractResult {
    pub char_map: String,
    pub upper_to_lower: String,
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

pub fn extract_tokenizer(
    tokenize_c_path: &str,
    output_path: &str,
) -> Result<TokenizerExtractResult, String> {
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
        w.include_local("csrc/sqlite_compat.h")
            .include_local("csrc/sqlite_tokens.h")
            .include_local("csrc/sqlite_keyword.h")
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
        .rename_function("sqlite3GetToken", "synq_sqlite3GetToken")
        .replace_all("TK_", "SYNTAQLITE_TK_")
        .finish();

    fs::write(output_path, output)
        .map_err(|e| format!("Failed to write {}: {}", output_path, e))?;

    Ok(TokenizerExtractResult {
        char_map: char_map.text,
        upper_to_lower: upper_to_lower.text,
    })
}

/// Generate parser by processing SQLite grammar and running lemon
///
/// This function:
/// 1. Reads the grammar file and writes to a temporary directory
/// 2. Extracts grammar tokens/rules using extract_grammar
/// 3. Writes the embedded lempar.c template
/// 4. Runs lemon via subprocess to generate parse.c and parse.h
/// 5. Copies parse.h (token definitions) to tokens_output if provided
///
/// # Arguments
/// * `grammar_path` - Path to the parse.y grammar file
/// * `output_dir` - Optional output directory (uses tempdir if not specified)
/// * `tokens_output` - Optional path to copy parse.h (token definitions) to
///
/// # Errors
/// Returns error if file operations or lemon execution fails
pub fn generate_parser(
    grammar_path: &str,
    output_dir: Option<&str>,
    tokens_output: Option<&str>,
) -> Result<(), String> {
    let grammar_bytes =
        fs::read(grammar_path).map_err(|e| format!("Failed to read {}: {}", grammar_path, e))?;
    // Create or use provided output directory
    let temp_dir: Option<tempfile::TempDir>;
    let work_dir = if let Some(dir) = output_dir {
        let path = Path::new(dir);
        fs::create_dir_all(path)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
        path
    } else {
        temp_dir = Some(
            tempfile::TempDir::new()
                .map_err(|e| format!("Failed to create temp directory: {}", e))?,
        );
        temp_dir.as_ref().unwrap().path()
    };

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

    // Copy token definitions if output path provided
    if let Some(tokens_out) = tokens_output {
        if let Some(parent) = Path::new(tokens_out).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create tokens output directory: {}", e))?;
        }

        // Read parse.h and rename TK_ to SYNTAQLITE_TK_
        let parse_h_content =
            fs::read_to_string(&parse_h).map_err(|e| format!("Failed to read parse.h: {}", e))?;
        let renamed_content = parse_h_content.replace("TK_", "SYNTAQLITE_TK_");

        fs::write(tokens_out, renamed_content)
            .map_err(|e| format!("Failed to write {}: {}", tokens_out, e))?;
    }

    Ok(())
}

/// Generate keyword hash lookup table
///
/// This function runs mkkeywordhash to generate optimized C code for keyword
/// recognition. The generated code is split into two files:
/// - sqlite_keyword_tables.h: Static keyword tables and hash data
/// - sqlite_keyword.c: The lookup function and character mapping arrays
///
/// # Arguments
/// * `output_path` - Path where the generated keyword hash C code will be written
///
/// # Errors
/// Returns error if mkkeywordhash execution fails or file writing fails
pub fn generate_keyword_hash(
    output_path: &str,
    extract_result: &TokenizerExtractResult,
) -> Result<(), String> {
    // Run mkkeywordhash as a subprocess and capture its output
    let output = std::process::Command::new(
        std::env::current_exe().map_err(|e| format!("Failed to get current executable: {}", e))?,
    )
    .arg("mkkeyword")
    .output()
    .map_err(|e| format!("Failed to spawn mkkeyword subprocess: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "mkkeyword failed with exit code: {}",
            output.status
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

    // Split the processed code into tables and function
    let (tables_content, function_content) = split_keyword_code(&processed_code, extract_result)?;

    // Write the tables header file
    let tables_path = "syntaqlite-parser/csrc/sqlite_keyword_tables.h";
    fs::write(tables_path, tables_content)
        .map_err(|e| format!("Failed to write {}: {}", tables_path, e))?;

    // Write the function C file
    fs::write(output_path, function_content)
        .map_err(|e| format!("Failed to write {}: {}", output_path, e))?;

    Ok(())
}

/// Split keyword code into tables header and function implementation
fn split_keyword_code(
    code: &str,
    extract_result: &TokenizerExtractResult,
) -> Result<(String, String), String> {
    // Use CExtractor to split by the function
    let extractor = c_extractor::CExtractor::new(code);
    let split = extractor.split_by_function("synq_sqlite3_keywordCode")?;

    let mut tables = c_writer::CWriter::new();
    tables.line("#ifndef SQLITE_KEYWORD_TABLES_H");
    tables.line("#define SQLITE_KEYWORD_TABLES_H");
    tables.newline();
    tables.include_local("csrc/sqlite_tokens.h");
    tables.newline();
    tables.fragment(&split.before);
    tables.newline();
    tables.line("#endif /* SQLITE_KEYWORD_TABLES_H */");

    // Build the function C file
    let mut func = c_writer::CWriter::new();
    func.include_local("csrc/sqlite_compat.h");
    func.include_local("csrc/sqlite_keyword_tables.h");
    func.newline();

    // Add char mapping arrays (upper_to_lower and char_map)
    func.fragment(&extract_result.upper_to_lower);
    func.newline();
    func.fragment(&extract_result.char_map);
    func.newline();

    // Add the function
    func.fragment(&split.function.text);
    func.newline();

    // Add anything after the function (if any)
    if !split.after.trim().is_empty() {
        func.fragment(&split.after);
        func.newline();
    }

    Ok((tables.finish(), func.finish()))
}

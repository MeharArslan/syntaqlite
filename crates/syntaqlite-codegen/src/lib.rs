pub mod grammar_parser;
pub mod c_writer;
pub mod c_extractor;
pub mod c_transform;

use std::fs;

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
        fs::write(output, c_code)
            .map_err(|e| format!("Failed to write {}: {}", output, e))?;
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
) -> Result<(), String> {
    use c_transform::ChangeNameTransform;

    let content = fs::read_to_string(tokenize_c_path)
        .map_err(|e| format!("Failed to read {}: {}", tokenize_c_path, e))?;

    let extractor = c_extractor::CExtractor::new(&content);

    let cc_defines = extractor.extract_defines_with_prefix("CC_")?;
    let array = extractor.extract_static_array("aiClass")?;
    let function = extractor.extract_function("sqlite3GetToken")?;
    let renamed = function.change_name("syntaqlite_get_token");

    let output = {
        let mut w = c_writer::CWriter::new();
        w.include_local("src/common/sqlite_compat.h")
            .newline()
            .fragment(&cc_defines)
            .newline()
            .fragment(&array)
            .newline()
            .fragment(&renamed);
        w.finish()
    };

    fs::write(output_path, output)
        .map_err(|e| format!("Failed to write {}: {}", output_path, e))?;

    Ok(())
}

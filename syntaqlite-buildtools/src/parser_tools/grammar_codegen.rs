// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fs;

use crate::util::grammar_parser;
use crate::util::c_writer::CWriter;

pub(crate) fn extract_grammar(input_path: &str, output_path: Option<&str>) -> Result<(), String> {
    let input_text =
        fs::read_to_string(input_path).map_err(|e| format!("Failed to read {input_path}: {e}"))?;

    let grammar = grammar_parser::LemonGrammar::parse(&input_text)
        .map_err(|e| format!("Parse error at {}:{}: {}", e.line, e.column, e.message))?;

    let c_code = generate_header(&grammar);

    if let Some(output) = output_path {
        fs::write(output, c_code).map_err(|e| format!("Failed to write {output}: {e}"))?;
    } else {
        print!("{c_code}");
    }

    Ok(())
}

fn generate_header(grammar: &grammar_parser::LemonGrammar) -> String {
    let mut w = CWriter::new();

    w.sqlite_file_header();

    let guard = "GRAMMAR_TOKENS_H";
    w.header_guard_start(guard);

    w.include_system("stdint.h");
    w.newline();

    w.extern_c_start();

    w.section("Token Types");
    let mut token_variants: Vec<(&str, Option<i32>)> = vec![("TOKEN_INVALID", Some(0))];
    for (i, token) in grammar.tokens.iter().enumerate() {
        token_variants.push((token.name, Some((i + 1) as i32)));
    }
    w.typedef_enum("TokenType", &token_variants);
    w.newline();

    w.comment(&format!("Total number of tokens: {}", grammar.tokens.len()));
    w.line(&format!("#define TOKEN_COUNT {}", grammar.tokens.len() + 1));
    w.newline();

    w.extern_c_end();
    w.newline();

    w.header_guard_end(guard);

    w.finish()
}

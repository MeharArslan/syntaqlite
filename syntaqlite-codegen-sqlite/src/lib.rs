// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

pub mod base_files;
pub mod grammar_codegen;
pub mod parser_pipeline;
mod sqlite_runtime_codegen;
pub mod tools;
pub mod util;

use std::fs;

use syntaqlite_codegen::dialect_codegen;
use syntaqlite_codegen::synq_parser;
pub use syntaqlite_codegen::{
    self, CodegenArtifacts, DialectNaming, OutputArtifact, OutputBucket, RustCodegenArtifacts,
    extract_token_defines, read_named_files_from_dir, sqlite_output_manifest,
};

pub struct TokenizerExtractResult {
    pub char_map: String,
    pub upper_to_lower: String,
}

pub struct CodegenRequest<'a> {
    pub dialect: &'a DialectNaming,
    pub y_files: &'a [(String, String)],
    pub synq_files: &'a [(String, String)],
    pub extra_keywords: &'a [String],
    pub parser_symbol_prefix: Option<&'a str>,
    pub include_rust: bool,
}

pub(crate) fn embedded_sqlite_tokenize_c() -> &'static str {
    include_str!(env!("SYNTAQLITE_SQLITE_TOKENIZE_C"))
}

pub(crate) fn embedded_sqlite_global_c() -> &'static str {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../third_party/src/sqlite/src/global.c"
    ))
}

pub(crate) fn embedded_sqlite_sqliteint_h() -> &'static str {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../third_party/src/sqlite/src/sqliteInt.h"
    ))
}

pub fn run_lemon(args: &[String]) -> ! {
    tools::lemon::run_lemon(args)
}

pub fn run_mkkeyword(args: &[String]) -> ! {
    tools::mkkeyword::run_mkkeyword(args)
}

/// Return the set of token names that are keywords in the base SQLite table.
pub fn base_keyword_token_names() -> std::collections::HashSet<String> {
    tools::mkkeyword::base_keyword_token_names()
}

pub(crate) fn extract_tokenizer(
    dialect: &str,
) -> Result<(String, TokenizerExtractResult), String> {
    sqlite_runtime_codegen::extract_tokenizer(
        embedded_sqlite_tokenize_c(),
        embedded_sqlite_global_c(),
        embedded_sqlite_sqliteint_h(),
        dialect,
    )
}

/// Extract terminal symbols (potential keywords) from extension `.y` grammar files.
pub fn extract_terminals_from_y(extension_y_contents: &[&str]) -> Vec<String> {
    sqlite_runtime_codegen::extract_terminals_from_y(extension_y_contents)
}

/// Generate keyword hash lookup as a single `.c` file.
pub fn generate_keyword_hash(
    extract_result: &TokenizerExtractResult,
    dialect: &str,
    extra_keywords: &[String],
) -> Result<String, String> {
    sqlite_runtime_codegen::generate_keyword_hash(extract_result, dialect, extra_keywords)
}

/// Generate the `sqlite_keyword.h` header.
pub fn generate_keyword_h() -> String {
    sqlite_runtime_codegen::generate_keyword_h()
}

/// Concatenate in-memory .y file contents (already sorted by caller).
pub fn concatenate_y_contents(files: &[(String, String)]) -> Result<Vec<u8>, String> {
    parser_pipeline::concatenate_y_contents(files)
}

/// Generate parser from in-memory .y file contents (merged base + extensions).
pub fn generate_parser_from_contents(
    y_files: &[(String, String)],
    parser_name: &str,
    output_dir: &str,
) -> Result<(), String> {
    parser_pipeline::generate_parser_from_contents(y_files, parser_name, output_dir)
}

pub fn extract_grammar(input_path: &str, output_path: Option<&str>) -> Result<(), String> {
    grammar_codegen::extract_grammar(input_path, output_path)
}

pub fn generate_parser(
    actions_dir: &str,
    parser_name: &str,
    output_dir: &str,
) -> Result<(), String> {
    parser_pipeline::generate_parser(actions_dir, parser_name, output_dir)
}

// --- Orchestration (from codegen_pipeline.rs) ---

fn parse_synq_items(synq_files: &[(String, String)]) -> Result<Vec<synq_parser::Item>, String> {
    let mut all_items = Vec::new();
    for (name, content) in synq_files {
        let items = synq_parser::parse_synq_file(content).map_err(|e| format!("{name}: {e}"))?;
        all_items.extend(items);
    }
    Ok(all_items)
}

pub fn generate_codegen_artifacts(
    request: &CodegenRequest<'_>,
) -> Result<CodegenArtifacts, String> {
    let parser_name = request.parser_symbol_prefix.unwrap_or("SynqSqliteParse");

    let all_items = parse_synq_items(request.synq_files)?;
    let ast_model = dialect_codegen::AstModel::new(&all_items);

    let work_dir =
        tempfile::TempDir::new().map_err(|e| format!("Failed to create temp directory: {e}"))?;
    parser_pipeline::generate_parser_from_contents(
        request.y_files,
        parser_name,
        work_dir.path().to_string_lossy().as_ref(),
    )?;
    let parse_h = fs::read_to_string(work_dir.path().join("parse.h"))
        .map_err(|e| format!("Failed to read parse.h: {e}"))?;
    let parse_c = fs::read_to_string(work_dir.path().join("parse.c"))
        .map_err(|e| format!("Failed to read parse.c: {e}"))?;

    let (tokenize_c, extract_result) = extract_tokenizer(request.dialect.name())?;
    let keyword_c = sqlite_runtime_codegen::generate_keyword_hash(
        &extract_result,
        request.dialect.name(),
        request.extra_keywords,
    )?;
    let keyword_h = sqlite_runtime_codegen::generate_keyword_h();

    let ast_nodes_h =
        dialect_codegen::generate_ast_nodes_header(&ast_model, request.dialect.name());
    let ast_builder_h =
        dialect_codegen::generate_ast_builder_header(&ast_model, request.dialect.name());
    let dialect_meta_h =
        dialect_codegen::generate_c_field_metadata(&ast_model, request.dialect.name())
            .map_err(|e| e.to_string())?;
    let dialect_fmt_h =
        dialect_codegen::generate_c_fmt_tables(&ast_model).map_err(|e| e.to_string())?;
    let token_defines = extract_token_defines(&parse_h);
    // Build keyword set from the base mkkeywordhash table + dialect extra keywords.
    let mut keyword_names = base_keyword_token_names();
    for kw in request.extra_keywords {
        keyword_names.insert(kw.to_uppercase());
    }
    let dialect_tokens_h =
        dialect_codegen::generate_token_categories_header(&token_defines, Some(&keyword_names));
    let parse_api_h = dialect_codegen::generate_parse_h(request.dialect.name());
    let dialect_c =
        dialect_codegen::generate_dialect_c(request.dialect.name(), Some(&token_defines));
    let dialect_h = dialect_codegen::generate_dialect_h(request.dialect.name());
    let dialect_dispatch_h = dialect_codegen::generate_dialect_dispatch_h(request.dialect.name());

    let rust = if request.include_rust {
        Some(RustCodegenArtifacts {
            tokens_rs: dialect_codegen::generate_rust_tokens(&token_defines[..]),
            ffi_rs: dialect_codegen::generate_rust_ffi_nodes(&ast_model),
            ast_rs: dialect_codegen::generate_rust_ast(&ast_model),
            lib_rs: dialect_codegen::generate_rust_lib(&request.dialect.dialect_symbol_fn_name()),
            wrappers_rs: dialect_codegen::generate_rust_wrappers(),
        })
    } else {
        None
    };

    Ok(CodegenArtifacts {
        parse_h,
        parse_api_h,
        parse_c,
        tokenize_c,
        keyword_c,
        keyword_h,
        ast_nodes_h,
        ast_builder_h,
        dialect_meta_h,
        dialect_fmt_h,
        dialect_tokens_h,
        dialect_c,
        dialect_h,
        dialect_dispatch_h,
        rust,
    })
}

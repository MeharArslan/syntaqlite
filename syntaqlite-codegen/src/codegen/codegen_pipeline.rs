// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fs;

use crate::dialect_codegen;
use crate::synq_parser;
use crate::{CodegenArtifacts, CodegenRequest, RustCodegenArtifacts};

fn parse_synq_items(synq_files: &[(String, String)]) -> Result<Vec<synq_parser::Item>, String> {
    let mut all_items = Vec::new();
    for (name, content) in synq_files {
        let items = synq_parser::parse_synq_file(content).map_err(|e| format!("{name}: {e}"))?;
        all_items.extend(items);
    }
    Ok(all_items)
}

pub(crate) fn generate_codegen_artifacts(
    request: &CodegenRequest<'_>,
) -> Result<CodegenArtifacts, String> {
    let y_files: Vec<(String, String)> = request
        .y_files
        .iter()
        .map(|(name, content)| {
            let rewritten = match request.parser_symbol_prefix {
                Some(prefix) => content.replace("SynqSqliteParse", prefix),
                None => content.clone(),
            };
            (name.clone(), rewritten)
        })
        .collect();

    let all_items = parse_synq_items(request.synq_files)?;
    let ast_model = dialect_codegen::AstModel::new(&all_items);

    let work_dir =
        tempfile::TempDir::new().map_err(|e| format!("Failed to create temp directory: {e}"))?;
    crate::codegen::parser_pipeline::generate_parser_from_contents(
        &y_files,
        work_dir.path().to_string_lossy().as_ref(),
    )?;
    let parse_h = fs::read_to_string(work_dir.path().join("parse.h"))
        .map_err(|e| format!("Failed to read parse.h: {e}"))?;
    let parse_c = fs::read_to_string(work_dir.path().join("parse.c"))
        .map_err(|e| format!("Failed to read parse.c: {e}"))?;

    let (tokenize_c, extract_result) = crate::codegen::sqlite_runtime_codegen::extract_tokenizer(
        request.tokenize_c_path,
        request.dialect.name(),
    )?;
    let keyword_c = crate::codegen::sqlite_runtime_codegen::generate_keyword_hash(
        &extract_result,
        request.dialect.name(),
        request.extra_keywords,
    )?;
    let keyword_h = crate::codegen::sqlite_runtime_codegen::generate_keyword_h();

    let ast_nodes_h =
        dialect_codegen::generate_ast_nodes_h_from_model(&ast_model, request.dialect.name());
    let ast_builder_h =
        dialect_codegen::generate_ast_builder_h_from_model(&ast_model, request.dialect.name());
    let dialect_meta_h = dialect_codegen::try_generate_c_field_meta_from_model_typed(
        &ast_model,
        request.dialect.name(),
    )
    .map_err(|e| e.to_string())?;
    let dialect_fmt_h = dialect_codegen::try_generate_c_fmt_arrays_typed(ast_model.items())
        .map_err(|e| e.to_string())?;
    let dialect_c = dialect_codegen::generate_dialect_c(request.dialect.name());
    let dialect_h = dialect_codegen::generate_dialect_h(request.dialect.name());
    let dialect_dispatch_h = dialect_codegen::generate_dialect_dispatch_h(request.dialect.name());

    let rust = if request.include_rust {
        let token_defines = crate::extract_token_defines(&parse_h);
        Some(RustCodegenArtifacts {
            tokens_rs: dialect_codegen::generate_rust_tokens(&token_defines),
            ffi_rs: dialect_codegen::generate_rust_ffi_nodes_from_model(&ast_model),
            ast_rs: dialect_codegen::generate_rust_ast_from_model(&ast_model),
            lib_rs: dialect_codegen::generate_rust_lib(&request.dialect.dialect_symbol_fn_name()),
            wrappers_rs: dialect_codegen::generate_rust_wrappers(),
        })
    } else {
        None
    };

    Ok(CodegenArtifacts {
        parse_h,
        parse_c,
        tokenize_c,
        keyword_c,
        keyword_h,
        ast_nodes_h,
        ast_builder_h,
        dialect_meta_h,
        dialect_fmt_h,
        dialect_c,
        dialect_h,
        dialect_dispatch_h,
        rust,
    })
}

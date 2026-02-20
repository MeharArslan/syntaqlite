// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fs;
use std::path::Path;

use crate::util::subprocess;

// Embed lempar.c template (needed by the library)
const LEMPAR_C: &[u8] = include_bytes!("../../sqlite/lempar.c");

pub(crate) fn generate_parser(actions_dir: &str, output_dir: &str) -> Result<(), String> {
    let grammar_bytes = concatenate_y_files(actions_dir)?;
    generate_parser_with_grammar_bytes(&grammar_bytes, output_dir)
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
pub(crate) fn generate_parser_from_contents(
    y_files: &[(String, String)],
    output_dir: &str,
) -> Result<(), String> {
    let grammar_bytes = concatenate_y_contents(y_files)?;
    generate_parser_with_grammar_bytes(&grammar_bytes, output_dir)
}

pub(crate) fn generate_parser_with_grammar_bytes(
    grammar_bytes: &[u8],
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

    crate::codegen::grammar_codegen::extract_grammar(parse_y_str, Some(extracted_grammar_str))?;

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

    Ok(())
}

fn run_lemon(template_arg: &str, parse_y_str: &str) -> Result<std::process::ExitStatus, String> {
    subprocess::self_subcommand("lemon")?
        .arg("-l")
        .arg(template_arg)
        .arg(parse_y_str)
        .status()
        .map_err(|e| format!("Failed to spawn lemon subprocess: {e}"))
}

/// Read all .y files from a directory, sort by name, and concatenate their contents.
fn concatenate_y_files(dir: &str) -> Result<Vec<u8>, String> {
    let y_files = crate::read_named_files_from_dir(dir, "y")?;
    if y_files.is_empty() {
        return Err(format!("No .y files found in {dir}"));
    }
    concatenate_y_contents(&y_files)
}

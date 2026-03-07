// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.
#![allow(missing_docs)] // ABI exports don't need rustdoc
#![cfg_attr(test, expect(clippy::unwrap_used, clippy::similar_names))]

use std::cell::RefCell;
use std::slice;

use serde::Serialize;

use syntaqlite::lsp::LspHost;
use syntaqlite::{FormatConfig, Formatter, KeywordCase, ValidationConfig};

thread_local! {
    static RESULT_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    /// Global LspHost reused across wasm calls. Invalidated when session context changes.
    static LSP_HOST: RefCell<Option<LspHost>> = const { RefCell::new(None) };
}

fn take_or_create_lsp_host() -> LspHost {
    LSP_HOST
        .with(|cell| cell.borrow_mut().take())
        .unwrap_or_else(LspHost::new)
}

fn store_lsp_host(lsp: LspHost) {
    LSP_HOST.with(|cell| *cell.borrow_mut() = Some(lsp));
}

fn invalidate_lsp_host() {
    LSP_HOST.with(|h| h.borrow_mut().take());
}

fn set_result(text: &str) {
    RESULT_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        buf.extend_from_slice(text.as_bytes());
    });
}

fn set_result_u32s(data: &[u32]) {
    RESULT_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        // SAFETY: u32 has no invalid bit patterns; reinterpreting as bytes is safe.
        let bytes = unsafe { slice::from_raw_parts(data.as_ptr().cast::<u8>(), data.len() * 4) };
        buf.extend_from_slice(bytes);
    });
}

fn decode_input(ptr: u32, len: u32) -> Result<String, String> {
    if len == 0 {
        return Ok(String::new());
    }
    if ptr == 0 {
        return Err("null input pointer".to_string());
    }
    // SAFETY: caller provides pointer/length in this module's linear memory.
    let bytes = unsafe { slice::from_raw_parts(ptr as *const u8, len as usize) };
    let source = std::str::from_utf8(bytes).map_err(|e| format!("invalid UTF-8 input: {e}"))?;
    Ok(source.to_string())
}

/// Runs `f`, catching any panic and writing `msg` to the result buffer on failure.
fn catch_unwind<F: FnOnce() -> i32>(f: F, msg: &'static str) -> i32 {
    if let Ok(result) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        result
    } else {
        set_result(msg);
        -1
    }
}

/// Unwraps a `Result`, writing the error to the result buffer and returning `$code` on failure.
macro_rules! try_wasm {
    ($expr:expr) => {
        try_wasm!($expr, 1)
    };
    ($expr:expr, $code:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => {
                set_result(&e.to_string());
                return $code;
            }
        }
    };
}

// ── Memory management ────────────────────────────────────────────────

fn alloc(len: u32) -> u32 {
    if len == 0 {
        return 0;
    }
    let mut buf = Vec::<u8>::with_capacity(len as usize);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr as u32
}

fn free(ptr: u32, len: u32) {
    if ptr == 0 || len == 0 {
        return;
    }
    // SAFETY: pointer/capacity pair must come from alloc().
    unsafe {
        let _ = Vec::<u8>::from_raw_parts(ptr as *mut u8, len as usize, len as usize);
    }
}

fn result_ptr() -> u32 {
    RESULT_BUF.with(|buf| {
        let buf = buf.borrow();
        if buf.is_empty() {
            0
        } else {
            buf.as_ptr() as u32
        }
    })
}

fn result_len() -> u32 {
    RESULT_BUF.with(|buf| buf.borrow().len() as u32)
}

fn result_free() {
    RESULT_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        buf.shrink_to_fit();
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_alloc(len: u32) -> u32 {
    alloc(len)
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_free(ptr: u32, len: u32) {
    free(ptr, len);
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_result_ptr() -> u32 {
    result_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_result_len() -> u32 {
    result_len()
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_result_free() {
    result_free();
}

// ── Formatter ────────────────────────────────────────────────────────

fn run_fmt(ptr: u32, len: u32, line_width: u32, keyword_case: u32, semicolons: u32) -> i32 {
    let source = try_wasm!(decode_input(ptr, len));
    let config = FormatConfig {
        line_width: if line_width == 0 {
            80
        } else {
            line_width as usize
        },
        keyword_case: match keyword_case {
            1 => KeywordCase::Upper,
            2 => KeywordCase::Lower,
            _ => KeywordCase::Preserve,
        },
        semicolons: semicolons != 0,
        ..Default::default()
    };
    let mut formatter = Formatter::with_config(&config);
    let sql = try_wasm!(formatter.format(&source).map_err(|e| e.to_string()));
    set_result(&sql);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_fmt(
    ptr: u32,
    len: u32,
    line_width: u32,
    keyword_case: u32,
    semicolons: u32,
) -> i32 {
    run_fmt(ptr, len, line_width, keyword_case, semicolons)
}

// ── Session context ──────────────────────────────────────────────────

fn run_set_session_context(ptr: u32, len: u32) -> i32 {
    let input = try_wasm!(decode_input(ptr, len));
    let mut lsp = take_or_create_lsp_host();
    try_wasm!(lsp.set_session_context_from_json(&input));
    store_lsp_host(lsp);
    0
}

fn run_set_session_context_ddl(ptr: u32, len: u32) -> i32 {
    let source = try_wasm!(decode_input(ptr, len));
    let mut lsp = take_or_create_lsp_host();
    lsp.set_session_context_from_ddl(&source);
    store_lsp_host(lsp);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_set_session_context(ptr: u32, len: u32) -> i32 {
    catch_unwind(
        || run_set_session_context(ptr, len),
        "wasm_set_session_context panicked",
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_clear_session_context() -> i32 {
    invalidate_lsp_host();
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_set_session_context_ddl(ptr: u32, len: u32) -> i32 {
    catch_unwind(
        || run_set_session_context_ddl(ptr, len),
        "wasm_set_session_context_ddl panicked",
    )
}

// ── Diagnostics / semantic tokens / completions ──────────────────────

const WASM_DOC_URI: &str = "wasm://input";

fn run_diagnostics(ptr: u32, len: u32, version: u32) -> i32 {
    let source = try_wasm!(decode_input(ptr, len), -1);
    let mut lsp = take_or_create_lsp_host();
    lsp.update_document(WASM_DOC_URI, version as i32, source);
    let all_diags = lsp.all_diagnostics(WASM_DOC_URI, &ValidationConfig::default());
    let total_count = all_diags.len();
    set_result(&serde_json::to_string(&all_diags).expect("diagnostic serialization failed"));
    store_lsp_host(lsp);
    total_count as i32
}

fn run_semantic_tokens(ptr: u32, len: u32, range_start: u32, range_end: u32, version: u32) -> i32 {
    let source = try_wasm!(decode_input(ptr, len), -1);
    let mut lsp = take_or_create_lsp_host();
    lsp.update_document(WASM_DOC_URI, version as i32, source);
    let range = if range_start == 0 && range_end == 0xFFFF_FFFF {
        None
    } else {
        Some((range_start as usize, range_end as usize))
    };
    let encoded = lsp.semantic_tokens_encoded(WASM_DOC_URI, range);
    let token_count = (encoded.len() / 5) as i32;
    set_result_u32s(&encoded);
    store_lsp_host(lsp);
    token_count
}

#[derive(Serialize)]
struct CompletionItem {
    label: String,
    kind: &'static str,
}

fn run_completions(ptr: u32, len: u32, offset: u32, version: u32) -> i32 {
    let source = try_wasm!(decode_input(ptr, len), -1);
    let mut lsp = take_or_create_lsp_host();
    lsp.update_document(WASM_DOC_URI, version as i32, source);
    let entries = lsp.completion_items(WASM_DOC_URI, offset as usize);
    let count = entries.len() as i32;
    let items: Vec<CompletionItem> = entries
        .into_iter()
        .map(|e| CompletionItem {
            label: e.label().to_string(),
            kind: e.kind().as_str(),
        })
        .collect();
    set_result(&serde_json::to_string(&items).expect("completions serialization failed"));
    store_lsp_host(lsp);
    count
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_diagnostics(ptr: u32, len: u32, version: u32) -> i32 {
    catch_unwind(
        || run_diagnostics(ptr, len, version),
        "wasm_diagnostics panicked",
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_semantic_tokens(
    ptr: u32,
    len: u32,
    range_start: u32,
    range_end: u32,
    version: u32,
) -> i32 {
    catch_unwind(
        || run_semantic_tokens(ptr, len, range_start, range_end, version),
        "wasm_semantic_tokens panicked",
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_completions(ptr: u32, len: u32, offset: u32, version: u32) -> i32 {
    catch_unwind(
        || run_completions(ptr, len, offset, version),
        "wasm_completions panicked",
    )
}

// ── Embedded SQL WASM exports ────────────────────────────────────────
//
// lang encoding: 0 = Python, 1 = TypeScript/JavaScript

use syntaqlite::embedded::{EmbeddedAnalyzer, EmbeddedFragment};

fn embedded_fragments(lang: u32, source: &str) -> Result<Vec<EmbeddedFragment>, String> {
    match lang {
        0 => Ok(syntaqlite::embedded::extract_python(source)),
        1 => Ok(syntaqlite::embedded::extract_typescript(source)),
        _ => Err(format!("unknown host language id: {lang}")),
    }
}

fn make_embedded_analyzer() -> EmbeddedAnalyzer {
    EmbeddedAnalyzer::new(syntaqlite::sqlite_dialect())
}

#[derive(Serialize)]
struct WasmHole {
    start: usize,
    end: usize,
    placeholder: String,
}

#[derive(Serialize)]
struct WasmFragment {
    start: usize,
    end: usize,
    sql: String,
    holes: Vec<WasmHole>,
}

fn run_embedded_extract(lang: u32, ptr: u32, len: u32) -> i32 {
    let source = try_wasm!(decode_input(ptr, len), -1);
    let fragments = try_wasm!(embedded_fragments(lang, &source), -1);
    let count = fragments.len() as i32;
    let items: Vec<WasmFragment> = fragments
        .iter()
        .map(|f| WasmFragment {
            start: f.sql_range.start,
            end: f.sql_range.end,
            sql: f.sql_text.clone(),
            holes: f
                .holes
                .iter()
                .map(|h| WasmHole {
                    start: h.host_range.start,
                    end: h.host_range.end,
                    placeholder: h.placeholder.clone(),
                })
                .collect(),
        })
        .collect();
    set_result(&serde_json::to_string(&items).expect("fragment serialization failed"));
    count
}

fn run_embedded_diagnostics(lang: u32, ptr: u32, len: u32) -> i32 {
    let source = try_wasm!(decode_input(ptr, len), -1);
    let fragments = try_wasm!(embedded_fragments(lang, &source), -1);
    let diags = make_embedded_analyzer().validate(&fragments);
    let count = diags.len() as i32;
    set_result(&serde_json::to_string(&diags).expect("embedded diagnostic serialization failed"));
    count
}

fn run_embedded_semantic_tokens(lang: u32, ptr: u32, len: u32) -> i32 {
    let source = try_wasm!(decode_input(ptr, len), -1);
    let fragments = try_wasm!(embedded_fragments(lang, &source), -1);
    let encoded = make_embedded_analyzer().semantic_tokens_encoded(&fragments, &source);
    let token_count = (encoded.len() / 5) as i32;
    set_result_u32s(&encoded);
    token_count
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_embedded_extract(lang: u32, ptr: u32, len: u32) -> i32 {
    catch_unwind(
        || run_embedded_extract(lang, ptr, len),
        "wasm_embedded_extract panicked",
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_embedded_diagnostics(lang: u32, ptr: u32, len: u32, _version: u32) -> i32 {
    catch_unwind(
        || run_embedded_diagnostics(lang, ptr, len),
        "wasm_embedded_diagnostics panicked",
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_embedded_semantic_tokens(
    lang: u32,
    ptr: u32,
    len: u32,
    _version: u32,
) -> i32 {
    catch_unwind(
        || run_embedded_semantic_tokens(lang, ptr, len),
        "wasm_embedded_semantic_tokens panicked",
    )
}

fn main() {}

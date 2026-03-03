// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![warn(unreachable_pub)]

use std::cell::{Cell, RefCell};
use std::slice;

use serde::Serialize;

use syntaqlite::Formatter;
use syntaqlite::embedded::{self, EmbeddedFragment};
use syntaqlite::{DatabaseCatalog, FormatConfig, KeywordCase, NodeRefJsonExt, ValidationConfig};
use syntaqlite_parser::{
    Cflags, Dialect, DialectEnv, Parser, cflag_table, parse_cflag_name, parse_sqlite_version,
};

thread_local! {
    static RESULT_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static DIALECT_PTR: Cell<u32> = const { Cell::new(0) };
    /// Global LspHost reused across wasm_diagnostics calls.
    /// Recreated when the dialect pointer changes.
    static LSP_HOST: RefCell<Option<LspHost>> = const { RefCell::new(None) };
    /// SQLite version applied to parser/formatter.
    static SQLITE_VERSION: Cell<i32> = const { Cell::new(i32::MAX) };
    /// Compile-time flags applied to parser/formatter.
    static SQLITE_CFLAGS: Cell<Cflags> = const { Cell::new(Cflags::new()) };
}

struct LspHost {
    dialect_ptr: u32,
    host: syntaqlite::lsp::LspHost<'static>,
}

fn take_or_create_lsp_host(dialect_ptr: u32) -> LspHost {
    let mut lsp = LSP_HOST.with(|cell| cell.borrow_mut().take());
    if lsp.as_ref().is_none_or(|h| h.dialect_ptr != dialect_ptr) {
        // SAFETY: the caller set a valid dialect pointer via wasm_set_dialect.
        let env = unsafe { DialectEnv::new(Dialect::from_raw(dialect_ptr as *const _)) };
        let env = apply_config(env);
        let host = syntaqlite::lsp::LspHost::with_dialect(env);
        lsp = Some(LspHost { dialect_ptr, host });
    }
    lsp.expect("LSP host must be initialized")
}

fn store_lsp_host(lsp: LspHost) {
    LSP_HOST.with(|cell| *cell.borrow_mut() = Some(lsp));
}

/// Apply the stored version/cflags to a `DialectEnv`.
fn apply_config(env: DialectEnv<'static>) -> DialectEnv<'static> {
    env.with_version(SQLITE_VERSION.with(|v| v.get()))
        .with_cflags(SQLITE_CFLAGS.with(|c| c.get()))
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
        let bytes = unsafe { slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) };
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

fn resolve_dialect() -> Result<DialectEnv<'static>, String> {
    let ptr = DIALECT_PTR.with(|p| p.get());
    if ptr == 0 {
        return Err("dialect pointer is not set; call wasm_set_dialect first".to_string());
    }
    // SAFETY: the caller must provide a valid pointer to a dialect descriptor.
    let env = unsafe { DialectEnv::new(Dialect::from_raw(ptr as *const _)) };
    Ok(apply_config(env))
}

/// Runs `f`, catching any panic and writing `msg` to the result buffer on failure.
fn catch_unwind<F: FnOnce() -> i32>(f: F, msg: &'static str) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(_) => {
            set_result(msg);
            -1
        }
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

// ── JSON AST dump ────────────────────────────────────────────────────

fn run_ast_json(ptr: u32, len: u32) -> i32 {
    let dialect = try_wasm!(resolve_dialect());
    let source = try_wasm!(decode_input(ptr, len));

    let parser = Parser::new(dialect);
    let mut cursor = parser.parse(&source);

    let mut ids = Vec::new();
    while let Some(result) = cursor.next_statement() {
        ids.push(try_wasm!(result).id());
    }

    // dump_json writes a raw JSON fragment per node; wrap in a JSON array manually
    // since the node dump format is not serde-based.
    let mut out = String::from('[');
    for (i, &id) in ids.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        cursor.node_ref(id).dump_json(&mut out);
    }
    out.push(']');
    set_result(&out);
    0
}

fn run_ast(ptr: u32, len: u32) -> i32 {
    let dialect = try_wasm!(resolve_dialect());
    let source = try_wasm!(decode_input(ptr, len));

    let parser = Parser::new(dialect);
    let mut cursor = parser.parse(&source);
    let mut out = String::new();
    let mut count = 0;

    while let Some(result) = cursor.next_statement() {
        let node = try_wasm!(result);
        if count > 0 {
            out.push_str("----\n");
        }
        node.dump(&mut out, 0);
        count += 1;
    }

    set_result(&out);
    0
}

fn run_fmt(ptr: u32, len: u32, line_width: u32, keyword_case: u32, semicolons: u32) -> i32 {
    let dialect = try_wasm!(resolve_dialect());
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
    let mut formatter = Formatter::with_config(dialect, &config);

    let sql = try_wasm!(formatter.format(&source));
    set_result(&sql);
    0
}

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
pub extern "C" fn wasm_set_dialect(dialect_ptr: u32) -> i32 {
    if dialect_ptr == 0 {
        set_result("null dialect pointer");
        return 1;
    }
    DIALECT_PTR.with(|p| p.set(dialect_ptr));
    invalidate_lsp_host();
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_clear_dialect() {
    DIALECT_PTR.with(|p| p.set(0));
    invalidate_lsp_host();
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_alloc(len: u32) -> u32 {
    alloc(len)
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_free(ptr: u32, len: u32) {
    free(ptr, len)
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ast(ptr: u32, len: u32) -> i32 {
    run_ast(ptr, len)
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_ast_json(ptr: u32, len: u32) -> i32 {
    run_ast_json(ptr, len)
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
    result_free()
}

// ── TypedDialectEnv config WASM exports ──────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn wasm_set_sqlite_version(ptr: u32, len: u32) -> i32 {
    let s = try_wasm!(decode_input(ptr, len));
    let ver = try_wasm!(parse_sqlite_version(&s));
    SQLITE_VERSION.with(|v| v.set(ver));
    invalidate_lsp_host();
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_set_cflag(ptr: u32, len: u32) -> i32 {
    let s = try_wasm!(decode_input(ptr, len));
    let idx = try_wasm!(parse_cflag_name(&s));
    SQLITE_CFLAGS.with(|c| {
        let mut cflags = c.get();
        cflags.set(idx);
        c.set(cflags);
    });
    invalidate_lsp_host();
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_clear_cflag(ptr: u32, len: u32) -> i32 {
    let s = try_wasm!(decode_input(ptr, len));
    let idx = try_wasm!(parse_cflag_name(&s));
    SQLITE_CFLAGS.with(|c| {
        let mut cflags = c.get();
        cflags.clear(idx);
        c.set(cflags);
    });
    invalidate_lsp_host();
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_clear_all_cflags() -> i32 {
    SQLITE_CFLAGS.with(|c| {
        let mut cflags = c.get();
        cflags.clear_all();
        c.set(cflags);
    });
    invalidate_lsp_host();
    0
}

// ── Session context WASM exports ─────────────────────────────────────

fn run_set_session_context(ptr: u32, len: u32) -> i32 {
    let input = try_wasm!(decode_input(ptr, len));
    let ctx = try_wasm!(DatabaseCatalog::from_json(&input));
    let dialect_ptr = try_wasm!(resolve_dialect().map(|_| DIALECT_PTR.with(|p| p.get())));
    let mut lsp = take_or_create_lsp_host(dialect_ptr);
    lsp.host.set_session_context(ctx);
    store_lsp_host(lsp);
    0
}

fn run_set_session_context_ddl(ptr: u32, len: u32) -> i32 {
    let dialect = try_wasm!(resolve_dialect());
    let source = try_wasm!(decode_input(ptr, len));
    let ctx = DatabaseCatalog::from_ddl(dialect, &source);
    let dialect_ptr = DIALECT_PTR.with(|p| p.get());
    let mut lsp = take_or_create_lsp_host(dialect_ptr);
    lsp.host.set_session_context(ctx);
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

// ── Cflag list ───────────────────────────────────────────────────────

#[derive(Serialize)]
struct CflagJson<'a> {
    name: &'a str,
    #[serde(rename = "minVersion")]
    min_version: i32,
    category: &'a str,
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_get_cflag_list() -> i32 {
    let table = cflag_table();
    let items: Vec<CflagJson> = table
        .iter()
        .map(|e| CflagJson {
            name: &e.suffix,
            min_version: e.min_version,
            category: &e.category,
        })
        .collect();
    set_result(&serde_json::to_string(&items).expect("cflag list serialization failed"));
    0
}

// ── Available functions ──────────────────────────────────────────────

#[derive(Serialize)]
struct AvailableFunction {
    name: String,
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_get_available_functions() -> i32 {
    let dialect_ptr = try_wasm!(resolve_dialect().map(|_| DIALECT_PTR.with(|p| p.get())), -1);
    let lsp = take_or_create_lsp_host(dialect_ptr);
    let names = lsp.host.available_function_names();
    let count = names.len() as i32;
    let items: Vec<AvailableFunction> = names
        .into_iter()
        .map(|name| AvailableFunction { name })
        .collect();
    set_result(&serde_json::to_string(&items).expect("available functions serialization failed"));
    store_lsp_host(lsp);
    count
}

// ── Diagnostics / semantic tokens / completions ──────────────────────

const WASM_DOC_URI: &str = "wasm://input";

fn run_diagnostics(ptr: u32, len: u32, version: u32) -> i32 {
    let dialect_ptr = try_wasm!(resolve_dialect().map(|_| DIALECT_PTR.with(|p| p.get())), -1);
    let source = try_wasm!(decode_input(ptr, len), -1);

    // Take the host out of the RefCell so we don't hold the borrow during
    // work that might panic. If it panics, we lose the host but don't poison
    // the RefCell.
    let mut lsp = take_or_create_lsp_host(dialect_ptr);
    lsp.host
        .update_document(WASM_DOC_URI, version as i32, source);
    let all_diags = lsp
        .host
        .all_diagnostics(WASM_DOC_URI, &ValidationConfig::default());
    let total_count = all_diags.len();
    set_result(&serde_json::to_string(&all_diags).expect("diagnostic serialization failed"));
    store_lsp_host(lsp);
    total_count as i32
}

fn run_semantic_tokens(ptr: u32, len: u32, range_start: u32, range_end: u32, version: u32) -> i32 {
    let dialect_ptr = try_wasm!(resolve_dialect().map(|_| DIALECT_PTR.with(|p| p.get())), -1);
    let source = try_wasm!(decode_input(ptr, len), -1);

    let mut lsp = take_or_create_lsp_host(dialect_ptr);
    lsp.host
        .update_document(WASM_DOC_URI, version as i32, source);

    let range = if range_start == 0 && range_end == 0xFFFFFFFF {
        None
    } else {
        Some((range_start as usize, range_end as usize))
    };
    let encoded = lsp.host.semantic_tokens_encoded(WASM_DOC_URI, range);
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
    let dialect_ptr = try_wasm!(resolve_dialect().map(|_| DIALECT_PTR.with(|p| p.get())), -1);
    let source = try_wasm!(decode_input(ptr, len), -1);

    let mut lsp = take_or_create_lsp_host(dialect_ptr);
    lsp.host
        .update_document(WASM_DOC_URI, version as i32, source);
    let entries = lsp.host.completion_items(WASM_DOC_URI, offset as usize);
    let count = entries.len() as i32;
    let items: Vec<CompletionItem> = entries
        .into_iter()
        .map(|e| CompletionItem {
            label: e.label,
            kind: e.kind.as_str(),
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

fn lang_to_fragments(lang: u32, source: &str) -> Result<Vec<EmbeddedFragment>, String> {
    match lang {
        0 => Ok(embedded::extract_python(source)),
        1 => Ok(embedded::extract_typescript(source)),
        _ => Err(format!("unknown embedded language: {lang}")),
    }
}

#[derive(Serialize)]
struct HoleJson {
    #[serde(rename = "hostRange")]
    host_range: [usize; 2],
    sql_offset: usize,
    placeholder: String,
}

#[derive(Serialize)]
struct FragmentJson {
    #[serde(rename = "sqlRange")]
    sql_range: [usize; 2],
    sql_text: String,
    holes: Vec<HoleJson>,
}

fn run_embedded_extract(lang: u32, ptr: u32, len: u32) -> i32 {
    let source = try_wasm!(decode_input(ptr, len), -1);
    let fragments = try_wasm!(lang_to_fragments(lang, &source), -1);
    let count = fragments.len() as i32;

    let items: Vec<FragmentJson> = fragments
        .iter()
        .map(|f| FragmentJson {
            sql_range: [f.sql_range.start, f.sql_range.end],
            sql_text: f.sql_text.clone(),
            holes: f
                .holes
                .iter()
                .map(|h| HoleJson {
                    host_range: [h.host_range.start, h.host_range.end],
                    sql_offset: h.sql_offset,
                    placeholder: h.placeholder.clone(),
                })
                .collect(),
        })
        .collect();
    set_result(&serde_json::to_string(&items).expect("fragment serialization failed"));
    count
}

fn run_embedded_diagnostics(lang: u32, ptr: u32, len: u32, _version: u32) -> i32 {
    let dialect = try_wasm!(resolve_dialect(), -1);
    let source = try_wasm!(decode_input(ptr, len), -1);
    let fragments = try_wasm!(lang_to_fragments(lang, &source), -1);

    // Syntax errors only — no session context means every table/column/function
    // would be flagged as unknown, so filter out semantic diagnostics entirely.
    let diags: Vec<_> = embedded::EmbeddedAnalyzer::new(dialect)
        .validate(&fragments)
        .into_iter()
        .filter(|d| d.message.is_parse_error())
        .collect();

    let total = diags.len() as i32;
    set_result(&serde_json::to_string(&diags).expect("diagnostic serialization failed"));
    total
}

fn run_embedded_semantic_tokens(lang: u32, ptr: u32, len: u32, _version: u32) -> i32 {
    let dialect = try_wasm!(resolve_dialect(), -1);
    let source = try_wasm!(decode_input(ptr, len), -1);
    let fragments = try_wasm!(lang_to_fragments(lang, &source), -1);

    let encoded =
        embedded::EmbeddedAnalyzer::new(dialect).semantic_tokens_encoded(&fragments, &source);
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
pub extern "C" fn wasm_embedded_diagnostics(lang: u32, ptr: u32, len: u32, version: u32) -> i32 {
    catch_unwind(
        || run_embedded_diagnostics(lang, ptr, len, version),
        "wasm_embedded_diagnostics panicked",
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_embedded_semantic_tokens(
    lang: u32,
    ptr: u32,
    len: u32,
    version: u32,
) -> i32 {
    catch_unwind(
        || run_embedded_semantic_tokens(lang, ptr, len, version),
        "wasm_embedded_semantic_tokens panicked",
    )
}

fn main() {}

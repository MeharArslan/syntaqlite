// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::{Cell, RefCell};
use std::slice;

use serde::Serialize;

use syntaqlite::dialect::{Cflags, DialectConfig, Dialect};
use syntaqlite::dialect::{cflag_table, parse_cflag_name, parse_sqlite_version};
use syntaqlite::embedded::{self, EmbeddedFragment};
use syntaqlite::raw::FfiDialect;
use syntaqlite::fmt::FormatConfig;
use syntaqlite::raw::RawParser;
use syntaqlite::validation::ValidationConfig;
use syntaqlite::Formatter;

thread_local! {
    static RESULT_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static DIALECT_PTR: Cell<u32> = const { Cell::new(0) };
    /// Global AnalysisHost reused across wasm_diagnostics calls.
    /// Recreated when the dialect pointer changes.
    static LSP_HOST: RefCell<Option<LspHost>> = const { RefCell::new(None) };
    /// Dialect config (version/cflags) applied to parser/formatter before each parse.
    static DIALECT_CONFIG: Cell<DialectConfig> = const { Cell::new(DialectConfig { sqlite_version: i32::MAX, cflags: Cflags::new() }) };
}

struct LspHost {
    dialect_ptr: u32,
    host: syntaqlite::lsp::AnalysisHost<'static>,
}

fn take_or_create_lsp_host(dialect_ptr: u32) -> LspHost {
    let mut lsp = LSP_HOST.with(|cell| cell.borrow_mut().take());
    if lsp.as_ref().is_none_or(|h| h.dialect_ptr != dialect_ptr) {
        let raw = dialect_ptr as *const FfiDialect;
        // SAFETY: the caller set a valid dialect pointer via wasm_set_dialect.
        let dialect = unsafe { Dialect::from_raw(raw) };
        let mut host = syntaqlite::lsp::AnalysisHost::with_dialect(dialect);
        host.set_dialect_config(get_dialect_config());
        lsp = Some(LspHost { dialect_ptr, host });
    }
    lsp.expect("LSP host must be initialized")
}

fn store_lsp_host(lsp: LspHost) {
    LSP_HOST.with(|cell| *cell.borrow_mut() = Some(lsp));
}

fn get_dialect_config() -> DialectConfig {
    DIALECT_CONFIG.with(|c| c.get())
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

fn resolve_dialect() -> Result<Dialect<'static>, String> {
    let ptr = DIALECT_PTR.with(|p| p.get());
    if ptr == 0 {
        return Err("dialect pointer is not set; call wasm_set_dialect first".to_string());
    }

    let raw = ptr as *const FfiDialect;
    // SAFETY: the caller must provide a valid pointer to a dialect descriptor.
    Ok(unsafe { Dialect::from_raw(raw) })
}

// ── JSON AST dump ────────────────────────────────────────────────────

fn run_ast_json(ptr: u32, len: u32) -> i32 {
    let dialect = match resolve_dialect() {
        Ok(d) => d,
        Err(e) => {
            set_result(&e);
            return 1;
        }
    };
    let source = match decode_input(ptr, len) {
        Ok(source) => source,
        Err(e) => {
            set_result(&e);
            return 1;
        }
    };

    let mut parser = RawParser::builder(&dialect).dialect_config(get_dialect_config()).build();
    let mut cursor = parser.parse(&source);

    let mut nodes = Vec::new();
    while let Some(result) = cursor.next_statement() {
        match result {
            Ok(node_ref) => nodes.push(node_ref),
            Err(e) => {
                set_result(&e.to_string());
                return 1;
            }
        }
    }

    // dump_json writes a raw JSON fragment per node; wrap in a JSON array manually
    // since the node dump format is not serde-based.
    let mut out = String::new();
    out.push('[');
    for (i, node) in nodes.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        node.dump_json(&mut out);
    }
    out.push(']');
    set_result(&out);
    0
}

fn run_ast(ptr: u32, len: u32) -> i32 {
    let dialect = match resolve_dialect() {
        Ok(d) => d,
        Err(e) => {
            set_result(&e);
            return 1;
        }
    };
    let source = match decode_input(ptr, len) {
        Ok(source) => source,
        Err(e) => {
            set_result(&e);
            return 1;
        }
    };

    let mut parser = RawParser::builder(&dialect).dialect_config(get_dialect_config()).build();
    let mut cursor = parser.parse(&source);
    let mut out = String::new();
    let mut count = 0;

    while let Some(result) = cursor.next_statement() {
        match result {
            Ok(node) => {
                if count > 0 {
                    out.push_str("----\n");
                }
                node.dump(&mut out, 0);
                count += 1;
            }
            Err(e) => {
                set_result(&e.to_string());
                return 1;
            }
        }
    }

    set_result(&out);
    0
}

fn run_fmt(ptr: u32, len: u32, line_width: u32, keyword_case: u32, semicolons: u32) -> i32 {
    let dialect = match resolve_dialect() {
        Ok(d) => d,
        Err(e) => {
            set_result(&e);
            return 1;
        }
    };
    let source = match decode_input(ptr, len) {
        Ok(source) => source,
        Err(e) => {
            set_result(&e);
            return 1;
        }
    };

    let config = FormatConfig::from_raw_params(line_width, keyword_case, semicolons);

    let mut formatter = Formatter::builder(&dialect)
        .format_config(config)
        .dialect_config(get_dialect_config())
        .build();

    match formatter.format(&source) {
        Ok(sql) => {
            set_result(&sql);
            0
        }
        Err(e) => {
            set_result(&e.to_string());
            1
        }
    }
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
    // Invalidate the cached LSP host so it's recreated with the new dialect.
    LSP_HOST.with(|h| h.borrow_mut().take());
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_clear_dialect() {
    DIALECT_PTR.with(|p| p.set(0));
    LSP_HOST.with(|h| h.borrow_mut().take());
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

// ── Dialect config WASM exports ──────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn wasm_set_sqlite_version(ptr: u32, len: u32) -> i32 {
    let s = match decode_input(ptr, len) {
        Ok(s) => s,
        Err(e) => {
            set_result(&e);
            return 1;
        }
    };
    match parse_sqlite_version(&s) {
        Ok(ver) => {
            DIALECT_CONFIG.with(|c| {
                let mut config = c.get();
                config.sqlite_version = ver;
                c.set(config);
            });
            invalidate_lsp_host();
            0
        }
        Err(e) => {
            set_result(&e);
            1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_set_cflag(ptr: u32, len: u32) -> i32 {
    let s = match decode_input(ptr, len) {
        Ok(s) => s,
        Err(e) => {
            set_result(&e);
            return 1;
        }
    };
    match parse_cflag_name(&s) {
        Ok(idx) => {
            DIALECT_CONFIG.with(|c| {
                let mut config = c.get();
                config.cflags.set(idx);
                c.set(config);
            });
            invalidate_lsp_host();
            0
        }
        Err(e) => {
            set_result(&e);
            1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_clear_cflag(ptr: u32, len: u32) -> i32 {
    let s = match decode_input(ptr, len) {
        Ok(s) => s,
        Err(e) => {
            set_result(&e);
            return 1;
        }
    };
    match parse_cflag_name(&s) {
        Ok(idx) => {
            DIALECT_CONFIG.with(|c| {
                let mut config = c.get();
                config.cflags.clear(idx);
                c.set(config);
            });
            invalidate_lsp_host();
            0
        }
        Err(e) => {
            set_result(&e);
            1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_clear_all_cflags() -> i32 {
    DIALECT_CONFIG.with(|c| {
        let mut config = c.get();
        config.cflags.clear_all();
        c.set(config);
    });
    invalidate_lsp_host();
    0
}

// ── Session context WASM exports ─────────────────────────────────────

fn run_set_session_context(ptr: u32, len: u32) -> i32 {
    let input = match decode_input(ptr, len) {
        Ok(s) => s,
        Err(e) => {
            set_result(&e);
            return 1;
        }
    };
    let ctx = match syntaqlite::validation::SessionContext::from_json(&input) {
        Ok(ctx) => ctx,
        Err(e) => {
            set_result(&e);
            return 1;
        }
    };

    let dialect_ptr = DIALECT_PTR.with(|p| p.get());
    if dialect_ptr == 0 {
        set_result("dialect pointer is not set; call wasm_set_dialect first");
        return 1;
    }
    let mut lsp = take_or_create_lsp_host(dialect_ptr);
    lsp.host.set_session_context(ctx);
    store_lsp_host(lsp);
    0
}

fn run_clear_session_context() -> i32 {
    // Invalidate the LSP host so it's recreated without context.
    invalidate_lsp_host();
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_set_session_context(ptr: u32, len: u32) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_set_session_context(ptr, len)
    })) {
        Ok(result) => result,
        Err(_) => {
            set_result("wasm_set_session_context panicked");
            1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_clear_session_context() -> i32 {
    run_clear_session_context()
}

fn run_set_session_context_ddl(ptr: u32, len: u32) -> i32 {
    let dialect = match resolve_dialect() {
        Ok(d) => d,
        Err(e) => {
            set_result(&e);
            return 1;
        }
    };
    let source = match decode_input(ptr, len) {
        Ok(s) => s,
        Err(e) => {
            set_result(&e);
            return 1;
        }
    };

    let ctx = syntaqlite::validation::SessionContext::from_ddl(
        &dialect,
        &source,
        Some(get_dialect_config()),
    );

    let dialect_ptr = DIALECT_PTR.with(|p| p.get());
    if dialect_ptr == 0 {
        set_result("dialect pointer is not set; call wasm_set_dialect first");
        return 1;
    }
    let mut lsp = take_or_create_lsp_host(dialect_ptr);
    lsp.host.set_session_context(ctx);
    store_lsp_host(lsp);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_set_session_context_ddl(ptr: u32, len: u32) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_set_session_context_ddl(ptr, len)
    })) {
        Ok(result) => result,
        Err(_) => {
            set_result("wasm_set_session_context_ddl panicked");
            1
        }
    }
}

#[derive(serde::Serialize)]
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

#[derive(Serialize)]
struct AvailableFunction {
    name: String,
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_get_available_functions() -> i32 {
    let dialect_ptr = DIALECT_PTR.with(|p| p.get());
    if dialect_ptr == 0 {
        set_result("dialect pointer is not set; call wasm_set_dialect first");
        return -1;
    }

    let lsp = take_or_create_lsp_host(dialect_ptr);
    let names = lsp.host.available_function_names();
    let count = names.len() as i32;
    let items: Vec<AvailableFunction> = names.into_iter().map(|name| AvailableFunction { name }).collect();
    set_result(&serde_json::to_string(&items).unwrap());

    store_lsp_host(lsp);
    count
}

const WASM_DOC_URI: &str = "wasm://input";

fn run_diagnostics(ptr: u32, len: u32, version: u32) -> i32 {
    let dialect_ptr = DIALECT_PTR.with(|p| p.get());
    if dialect_ptr == 0 {
        set_result("dialect pointer is not set; call wasm_set_dialect first");
        return -1;
    }
    let source = match decode_input(ptr, len) {
        Ok(source) => source,
        Err(e) => {
            set_result(&e);
            return -1;
        }
    };

    // Take the host out of the RefCell so we don't hold the borrow during
    // work that might panic. If it panics, we lose the host but don't poison
    // the RefCell.
    let mut lsp = take_or_create_lsp_host(dialect_ptr);
    lsp.host
        .update_document(WASM_DOC_URI, version as i32, source);
    let all_diags = lsp.host.all_diagnostics(WASM_DOC_URI, &ValidationConfig::default());
    let total_count = all_diags.len();
    set_result(&serde_json::to_string(&all_diags).expect("diagnostic serialization failed"));

    // Put the host back for reuse.
    store_lsp_host(lsp);

    total_count as i32
}

fn run_semantic_tokens(ptr: u32, len: u32, range_start: u32, range_end: u32, version: u32) -> i32 {
    let dialect_ptr = DIALECT_PTR.with(|p| p.get());
    if dialect_ptr == 0 {
        set_result("dialect pointer is not set; call wasm_set_dialect first");
        return -1;
    }
    let source = match decode_input(ptr, len) {
        Ok(source) => source,
        Err(e) => {
            set_result(&e);
            return -1;
        }
    };

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

    // Write the Vec<u32> as raw bytes into RESULT_BUF.
    RESULT_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        let bytes: &[u8] =
            unsafe { slice::from_raw_parts(encoded.as_ptr() as *const u8, encoded.len() * 4) };
        buf.extend_from_slice(bytes);
    });

    store_lsp_host(lsp);

    token_count
}

fn run_completions(ptr: u32, len: u32, offset: u32, version: u32) -> i32 {
    let dialect_ptr = DIALECT_PTR.with(|p| p.get());
    if dialect_ptr == 0 {
        set_result("dialect pointer is not set; call wasm_set_dialect first");
        return -1;
    }
    let source = match decode_input(ptr, len) {
        Ok(source) => source,
        Err(e) => {
            set_result(&e);
            return -1;
        }
    };

    #[derive(Serialize)]
    struct CompletionItem {
        label: String,
        kind: &'static str,
    }

    let mut lsp = take_or_create_lsp_host(dialect_ptr);
    lsp.host
        .update_document(WASM_DOC_URI, version as i32, source);
    let entries = lsp.host.completion_items(WASM_DOC_URI, offset as usize);
    let count = entries.len() as i32;
    let items: Vec<CompletionItem> = entries
        .into_iter()
        .map(|e| CompletionItem {
            label: e.label,
            kind: match e.kind {
                syntaqlite::lsp::CompletionKind::Keyword => "keyword",
                syntaqlite::lsp::CompletionKind::Function => "function",
            },
        })
        .collect();
    set_result(&serde_json::to_string(&items).unwrap());
    store_lsp_host(lsp);
    count
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_semantic_tokens(
    ptr: u32,
    len: u32,
    range_start: u32,
    range_end: u32,
    version: u32,
) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_semantic_tokens(ptr, len, range_start, range_end, version)
    })) {
        Ok(result) => result,
        Err(_) => {
            set_result("wasm_semantic_tokens panicked");
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_completions(ptr: u32, len: u32, offset: u32, version: u32) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_completions(ptr, len, offset, version)
    })) {
        Ok(result) => result,
        Err(_) => {
            set_result("wasm_completions panicked");
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_diagnostics(ptr: u32, len: u32, version: u32) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_diagnostics(ptr, len, version)
    })) {
        Ok(result) => result,
        Err(_) => {
            set_result("wasm_diagnostics panicked");
            -1
        }
    }
}

// ── Embedded SQL WASM exports ────────────────────────────────────────

fn extract_fragments(lang: u32, source: &str) -> Result<Vec<EmbeddedFragment>, String> {
    match lang {
        0 => Ok(embedded::extract_python(source)),
        1 => Ok(embedded::extract_typescript(source)),
        _ => Err(format!("unknown embedded language: {lang}")),
    }
}

fn run_embedded_extract(lang: u32, ptr: u32, len: u32) -> i32 {
    let source = match decode_input(ptr, len) {
        Ok(s) => s,
        Err(e) => {
            set_result(&e);
            return -1;
        }
    };

    let fragments = match extract_fragments(lang, &source) {
        Ok(f) => f,
        Err(e) => {
            set_result(&e);
            return -1;
        }
    };

    let count = fragments.len() as i32;

    #[derive(serde::Serialize)]
    struct HoleJson {
        #[serde(rename = "hostRange")]
        host_range: [usize; 2],
        sql_offset: usize,
        placeholder: String,
    }

    #[derive(serde::Serialize)]
    struct FragmentJson {
        #[serde(rename = "sqlRange")]
        sql_range: [usize; 2],
        sql_text: String,
        holes: Vec<HoleJson>,
    }

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
    let dialect = match resolve_dialect() {
        Ok(d) => d,
        Err(e) => {
            set_result(&e);
            return -1;
        }
    };
    let source = match decode_input(ptr, len) {
        Ok(s) => s,
        Err(e) => {
            set_result(&e);
            return -1;
        }
    };

    let fragments = match extract_fragments(lang, &source) {
        Ok(f) => f,
        Err(e) => {
            set_result(&e);
            return -1;
        }
    };

    // Syntax errors only — no session context means every table/column/function
    // would be flagged as unknown, so filter out semantic diagnostics entirely.
    let config = ValidationConfig::default();
    let all_diags = embedded::validate_embedded(&dialect, &fragments, &[], &config);
    let diags: Vec<_> = all_diags
        .into_iter()
        .filter(|d| d.message.is_parse_error())
        .collect();

    let total = diags.len() as i32;
    set_result(&serde_json::to_string(&diags).expect("diagnostic serialization failed"));
    total
}

fn run_embedded_semantic_tokens(lang: u32, ptr: u32, len: u32, _version: u32) -> i32 {
    let dialect = match resolve_dialect() {
        Ok(d) => d,
        Err(e) => {
            set_result(&e);
            return -1;
        }
    };
    let source = match decode_input(ptr, len) {
        Ok(s) => s,
        Err(e) => {
            set_result(&e);
            return -1;
        }
    };

    let fragments = match extract_fragments(lang, &source) {
        Ok(f) => f,
        Err(e) => {
            set_result(&e);
            return -1;
        }
    };

    let result = embedded::embedded_semantic_tokens_encoded(&dialect, &fragments, &source);
    let token_count = (result.len() / 5) as i32;

    RESULT_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        let bytes: &[u8] =
            unsafe { slice::from_raw_parts(result.as_ptr() as *const u8, result.len() * 4) };
        buf.extend_from_slice(bytes);
    });

    token_count
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_embedded_extract(lang: u32, ptr: u32, len: u32) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_embedded_extract(lang, ptr, len)
    })) {
        Ok(result) => result,
        Err(_) => {
            set_result("wasm_embedded_extract panicked");
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_embedded_diagnostics(
    lang: u32,
    ptr: u32,
    len: u32,
    version: u32,
) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_embedded_diagnostics(lang, ptr, len, version)
    })) {
        Ok(result) => result,
        Err(_) => {
            set_result("wasm_embedded_diagnostics panicked");
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_embedded_semantic_tokens(
    lang: u32,
    ptr: u32,
    len: u32,
    version: u32,
) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_embedded_semantic_tokens(lang, ptr, len, version)
    })) {
        Ok(result) => result,
        Err(_) => {
            set_result("wasm_embedded_semantic_tokens panicked");
            -1
        }
    }
}

fn main() {}

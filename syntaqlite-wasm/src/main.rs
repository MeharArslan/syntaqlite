// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::{Cell, RefCell};
use std::slice;

use syntaqlite_runtime::dialect::ffi::{
    self as dialect_ffi, FIELD_BOOL, FIELD_ENUM, FIELD_FLAGS, FIELD_NODE_ID, FIELD_SPAN,
};
use syntaqlite_runtime::fmt::{FormatConfig, Formatter, KeywordCase};
use syntaqlite_runtime::parser::{CursorBase, SourceSpan};
use syntaqlite_runtime::{Dialect, NodeId, Parser};

thread_local! {
    static RESULT_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static DIALECT_PTR: Cell<u32> = const { Cell::new(0) };
    /// Global AnalysisHost reused across wasm_diagnostics calls.
    /// Recreated when the dialect pointer changes.
    static LSP_HOST: RefCell<Option<LspHost>> = const { RefCell::new(None) };
}

struct LspHost {
    dialect_ptr: u32,
    host: syntaqlite_lsp::AnalysisHost<'static>,
}

fn take_or_create_lsp_host(dialect_ptr: u32) -> LspHost {
    let mut lsp = LSP_HOST.with(|cell| cell.borrow_mut().take());
    if lsp.as_ref().is_none_or(|h| h.dialect_ptr != dialect_ptr) {
        let raw = dialect_ptr as *const dialect_ffi::Dialect;
        // SAFETY: the caller set a valid dialect pointer via wasm_set_dialect.
        let dialect = unsafe { Dialect::from_raw(raw) };
        lsp = Some(LspHost {
            dialect_ptr,
            host: syntaqlite_lsp::AnalysisHost::new(dialect),
        });
    }
    lsp.expect("LSP host must be initialized")
}

fn store_lsp_host(lsp: LspHost) {
    LSP_HOST.with(|cell| *cell.borrow_mut() = Some(lsp));
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

    let raw = ptr as *const dialect_ffi::Dialect;
    // SAFETY: the caller must provide a valid pointer to a dialect descriptor.
    Ok(unsafe { Dialect::from_raw(raw) })
}

// ── JSON AST dump ────────────────────────────────────────────────────

fn json_escape(out: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < '\x20' => {
                use std::fmt::Write;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
}

fn dump_node_json(out: &mut String, dialect: &Dialect, cursor: &CursorBase, id: NodeId) {
    let Some((ptr, tag)) = cursor.node_ptr(id) else {
        out.push_str("null");
        return;
    };

    let name = dialect.node_name(tag);

    if dialect.is_list(tag) {
        let children = cursor.list_children(id, dialect).unwrap_or(&[]);
        out.push_str("{\"type\":\"list\",\"name\":\"");
        json_escape(out, name);
        out.push_str("\",\"count\":");
        out.push_str(&children.len().to_string());
        out.push_str(",\"children\":[");
        for (i, &child_id) in children.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            dump_node_json(out, dialect, cursor, child_id);
        }
        out.push_str("]}");
        return;
    }

    let meta = dialect.field_meta(tag);
    let source = cursor.source();

    out.push_str("{\"type\":\"node\",\"name\":\"");
    json_escape(out, name);
    out.push_str("\",\"fields\":[");

    for (i, m) in meta.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }

        let label = unsafe { std::ffi::CStr::from_ptr(m.name) }
            .to_str()
            .unwrap_or("?");

        unsafe {
            let field_ptr = ptr.add(m.offset as usize);
            match m.kind {
                FIELD_NODE_ID => {
                    let child_id = NodeId(*(field_ptr as *const u32));
                    out.push_str("{\"label\":\"");
                    json_escape(out, label);
                    out.push_str("\",\"kind\":\"node\",\"child\":");
                    if child_id.is_null() {
                        out.push_str("null");
                    } else {
                        dump_node_json(out, dialect, cursor, child_id);
                    }
                    out.push('}');
                }
                FIELD_SPAN => {
                    let span = &*(field_ptr as *const SourceSpan);
                    out.push_str("{\"label\":\"");
                    json_escape(out, label);
                    out.push_str("\",\"kind\":\"span\",\"value\":");
                    if span.is_empty() {
                        out.push_str("null");
                    } else {
                        out.push('"');
                        json_escape(out, span.as_str(source));
                        out.push('"');
                    }
                    out.push('}');
                }
                FIELD_BOOL => {
                    let val = *(field_ptr as *const u32) != 0;
                    out.push_str("{\"label\":\"");
                    json_escape(out, label);
                    out.push_str("\",\"kind\":\"bool\",\"value\":");
                    out.push_str(if val { "true" } else { "false" });
                    out.push('}');
                }
                FIELD_ENUM => {
                    let val = *(field_ptr as *const u32);
                    out.push_str("{\"label\":\"");
                    json_escape(out, label);
                    out.push_str("\",\"kind\":\"enum\",\"value\":");
                    // C dump: display[val] where val is the raw enum ordinal.
                    let display_count = m.display_count as usize;
                    if (val as usize) < display_count && !m.display.is_null() {
                        let display_ptr = *m.display.add(val as usize);
                        if !display_ptr.is_null() {
                            let cstr = std::ffi::CStr::from_ptr(display_ptr);
                            let s = cstr.to_str().unwrap_or("?");
                            out.push('"');
                            json_escape(out, s);
                            out.push('"');
                        } else {
                            out.push_str("null");
                        }
                    } else {
                        out.push_str("null");
                    }
                    out.push('}');
                }
                FIELD_FLAGS => {
                    let val = *(field_ptr as *const u8);
                    out.push_str("{\"label\":\"");
                    json_escape(out, label);
                    out.push_str("\",\"kind\":\"flags\",\"value\":[");
                    let display_count = m.display_count as usize;
                    let mut first = true;
                    for bit in 0..8u8 {
                        if val & (1 << bit) != 0 {
                            if !first {
                                out.push(',');
                            }
                            first = false;
                            if (bit as usize) < display_count {
                                let display_ptr = *m.display.add(bit as usize);
                                if !display_ptr.is_null() {
                                    let cstr = std::ffi::CStr::from_ptr(display_ptr);
                                    let s = cstr.to_str().unwrap_or("?");
                                    out.push('"');
                                    json_escape(out, s);
                                    out.push('"');
                                } else {
                                    out.push_str(&(1u32 << bit).to_string());
                                }
                            } else {
                                out.push_str(&(1u32 << bit).to_string());
                            }
                        }
                    }
                    out.push_str("]}");
                }
                _ => {
                    out.push_str("{\"label\":\"");
                    json_escape(out, label);
                    out.push_str("\",\"kind\":\"unknown\",\"value\":null}");
                }
            }
        }
    }

    out.push_str("]}");
}

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

    let mut parser = Parser::new(&dialect);
    let mut cursor = parser.parse(&source);
    let mut out = String::new();
    out.push('[');
    let mut count = 0;

    while let Some(result) = cursor.next_statement() {
        match result {
            Ok(root_id) => {
                if count > 0 {
                    out.push(',');
                }
                dump_node_json(&mut out, &dialect, cursor.base(), root_id);
                count += 1;
            }
            Err(e) => {
                set_result(&e.to_string());
                return 1;
            }
        }
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

    let mut parser = Parser::new(&dialect);
    let mut cursor = parser.parse(&source);
    let mut out = String::new();
    let mut count = 0;

    while let Some(result) = cursor.next_statement() {
        match result {
            Ok(root_id) => {
                if count > 0 {
                    out.push_str("----\n");
                }
                cursor.dump_node(root_id, &mut out, 0);
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

    let mut formatter = match Formatter::with_config(&dialect, config) {
        Ok(formatter) => formatter,
        Err(e) => {
            set_result(e);
            return 1;
        }
    };

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
    let diags = lsp.host.diagnostics(WASM_DOC_URI);

    let count = diags.len() as i32;

    // Serialize diagnostics as JSON array (no serde in WASM).
    let mut out = String::new();
    out.push('[');
    for (i, d) in diags.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"startOffset\":");
        out.push_str(&d.start_offset.to_string());
        out.push_str(",\"endOffset\":");
        out.push_str(&d.end_offset.to_string());
        out.push_str(",\"message\":\"");
        json_escape(&mut out, &d.message);
        out.push_str("\",\"severity\":\"");
        out.push_str(match d.severity {
            syntaqlite_lsp::Severity::Error => "error",
            syntaqlite_lsp::Severity::Warning => "warning",
            syntaqlite_lsp::Severity::Info => "info",
            syntaqlite_lsp::Severity::Hint => "hint",
        });
        out.push_str("\"}");
    }
    out.push(']');
    set_result(&out);

    // Put the host back for reuse.
    store_lsp_host(lsp);

    count
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

fn main() {}

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::slice;

use serde::Serialize;

use syntaqlite::dialect::ffi::{self as dialect_ffi, DialectConfig};
use syntaqlite::fmt::{FormatConfig, Formatter, KeywordCase};
use syntaqlite::parser::{CursorBase, FieldVal};
use syntaqlite::{Dialect, NodeId, Parser};

thread_local! {
    static RESULT_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static DIALECT_PTR: Cell<u32> = const { Cell::new(0) };
    /// Global AnalysisHost reused across wasm_diagnostics calls.
    /// Recreated when the dialect pointer changes.
    static LSP_HOST: RefCell<Option<LspHost>> = const { RefCell::new(None) };
    /// Dialect config (version/cflags) applied to parser/formatter before each parse.
    static DIALECT_CONFIG: Cell<DialectConfig> = const { Cell::new(DialectConfig { sqlite_version: i32::MAX, cflags: dialect_ffi::Cflags::new() }) };
}

struct LspHost {
    dialect_ptr: u32,
    host: syntaqlite::lsp::AnalysisHost<'static>,
}

fn take_or_create_lsp_host(dialect_ptr: u32) -> LspHost {
    let mut lsp = LSP_HOST.with(|cell| cell.borrow_mut().take());
    if lsp.as_ref().is_none_or(|h| h.dialect_ptr != dialect_ptr) {
        let raw = dialect_ptr as *const dialect_ffi::Dialect;
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

/// Serde-serializable AST node for JSON output.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum AstNode<'a> {
    #[serde(rename = "list")]
    List {
        name: &'a str,
        count: usize,
        children: Vec<AstNode<'a>>,
    },
    #[serde(rename = "node")]
    Node {
        name: &'a str,
        fields: Vec<AstField<'a>>,
    },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum AstField<'a> {
    #[serde(rename = "node")]
    Node {
        label: &'a str,
        child: Option<AstNode<'a>>,
    },
    #[serde(rename = "span")]
    Span {
        label: &'a str,
        value: Option<&'a str>,
    },
    #[serde(rename = "bool")]
    Bool { label: &'a str, value: bool },
    #[serde(rename = "enum")]
    Enum {
        label: &'a str,
        value: Option<&'a str>,
    },
    #[serde(rename = "flags")]
    Flags {
        label: &'a str,
        value: Vec<FlagValue<'a>>,
    },
}

#[derive(Serialize)]
#[serde(untagged)]
enum FlagValue<'a> {
    Named(&'a str),
    Numeric(u32),
}

fn build_ast_node<'a>(
    dialect: &'a Dialect,
    cursor: &'a CursorBase,
    id: NodeId,
) -> Option<AstNode<'a>> {
    let tag = cursor.reader().node_tag(id)?;
    let name = dialect.node_name(tag);

    if dialect.is_list(tag) {
        let children = cursor.list_children(id, dialect).unwrap_or(&[]);
        let child_nodes: Vec<_> = children
            .iter()
            .map(|&child_id| {
                build_ast_node(dialect, cursor, child_id).unwrap_or(AstNode::Node {
                    name: "null",
                    fields: vec![],
                })
            })
            .collect();
        return Some(AstNode::List {
            name,
            count: child_nodes.len(),
            children: child_nodes,
        });
    }

    let meta = dialect.field_meta(tag);
    let (_, fields) = cursor.reader().extract_fields(id, dialect)?;

    let ast_fields: Vec<_> = meta
        .iter()
        .zip(fields.iter())
        .map(|(m, fv)| {
            // SAFETY: m.name is a valid NUL-terminated C string from codegen.
            let label = unsafe { m.name_str() };

            match fv {
                FieldVal::NodeId(child_id) => AstField::Node {
                    label,
                    child: if child_id.is_null() {
                        None
                    } else {
                        build_ast_node(dialect, cursor, *child_id)
                    },
                },
                FieldVal::Span(text, _) => AstField::Span {
                    label,
                    value: if text.is_empty() { None } else { Some(text) },
                },
                FieldVal::Bool(val) => AstField::Bool { label, value: *val },
                FieldVal::Enum(val) => AstField::Enum {
                    label,
                    // SAFETY: m.display is a valid C array from codegen.
                    value: unsafe { m.display_name(*val as usize) },
                },
                FieldVal::Flags(val) => {
                    let mut flags = Vec::new();
                    for bit in 0..8u8 {
                        if val & (1 << bit) != 0 {
                            // SAFETY: m.display is a valid C array from codegen.
                            match unsafe { m.display_name(bit as usize) } {
                                Some(s) => flags.push(FlagValue::Named(s)),
                                None => flags.push(FlagValue::Numeric(1u32 << bit)),
                            }
                        }
                    }
                    AstField::Flags {
                        label,
                        value: flags,
                    }
                }
            }
        })
        .collect();

    Some(AstNode::Node {
        name,
        fields: ast_fields,
    })
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

    let mut parser = Parser::with_dialect(&dialect);
    let config = get_dialect_config();
    parser.set_dialect_config(&config);
    let mut cursor = parser.parse(&source);

    let mut root_ids = Vec::new();
    while let Some(result) = cursor.next_statement() {
        match result {
            Ok(root_id) => root_ids.push(root_id),
            Err(e) => {
                set_result(&e.to_string());
                return 1;
            }
        }
    }

    let nodes: Vec<_> = root_ids
        .iter()
        .map(|&id| build_ast_node(&dialect, cursor.base(), id))
        .collect();

    match serde_json::to_string(&nodes) {
        Ok(json) => {
            set_result(&json);
            0
        }
        Err(e) => {
            set_result(&e.to_string());
            1
        }
    }
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

    let mut parser = Parser::with_dialect(&dialect);
    let config = get_dialect_config();
    parser.set_dialect_config(&config);
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

    let mut formatter = match Formatter::with_dialect_config(&dialect, config) {
        Ok(formatter) => formatter,
        Err(e) => {
            set_result(e);
            return 1;
        }
    };

    let dialect_config = get_dialect_config();
    formatter.set_dialect_config(&dialect_config);

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
    match dialect_ffi::parse_sqlite_version(&s) {
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
    match dialect_ffi::parse_cflag_name(&s) {
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
    match dialect_ffi::parse_cflag_name(&s) {
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

#[derive(serde::Deserialize)]
struct SessionContextJson {
    #[serde(default)]
    tables: Vec<TableJson>,
    #[serde(default)]
    views: Vec<ViewJson>,
    #[serde(default)]
    functions: Vec<FunctionJson>,
}

#[derive(serde::Deserialize)]
struct TableJson {
    name: String,
    #[serde(default)]
    columns: Vec<String>,
}

#[derive(serde::Deserialize)]
struct ViewJson {
    name: String,
    #[serde(default)]
    columns: Vec<String>,
}

#[derive(serde::Deserialize)]
struct FunctionJson {
    name: String,
    args: Option<usize>,
}

impl From<SessionContextJson> for syntaqlite::validation::SessionContext {
    fn from(json: SessionContextJson) -> Self {
        use syntaqlite::validation::{ColumnDef, RelationDef, RelationKind};
        let make_columns = |cols: Vec<String>| -> Vec<ColumnDef> {
            cols.into_iter()
                .map(|c| ColumnDef {
                    name: c,
                    type_name: None,
                    is_primary_key: false,
                    is_nullable: true,
                })
                .collect()
        };
        let relations = json
            .tables
            .into_iter()
            .map(|t| RelationDef {
                name: t.name,
                columns: make_columns(t.columns),
                kind: RelationKind::Table,
            })
            .chain(json.views.into_iter().map(|v| RelationDef {
                name: v.name,
                columns: make_columns(v.columns),
                kind: RelationKind::View,
            }))
            .collect();
        syntaqlite::validation::SessionContext {
            relations,
            functions: json
                .functions
                .into_iter()
                .map(|f| syntaqlite::validation::FunctionDef {
                    name: f.name,
                    args: f.args,
                    description: None,
                })
                .collect(),
        }
    }
}

fn run_set_session_context(ptr: u32, len: u32) -> i32 {
    let input = match decode_input(ptr, len) {
        Ok(s) => s,
        Err(e) => {
            set_result(&e);
            return 1;
        }
    };
    let json: SessionContextJson = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(e) => {
            set_result(&format!("invalid session context JSON: {e}"));
            return 1;
        }
    };
    let ctx: syntaqlite::validation::SessionContext = json.into();

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

    let mut parser = Parser::with_dialect(&dialect);
    let config = get_dialect_config();
    parser.set_dialect_config(&config);
    let mut cursor = parser.parse(&source);

    let mut stmt_ids = Vec::new();
    while let Some(result) = cursor.next_statement() {
        match result {
            Ok(id) => stmt_ids.push(id),
            Err(e) => {
                set_result(&e.to_string());
                return 1;
            }
        }
    }

    let ctx =
        syntaqlite::validation::SessionContext::from_stmts(cursor.reader(), &stmt_ids, &dialect);

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

#[unsafe(no_mangle)]
pub extern "C" fn wasm_get_cflag_list() -> i32 {
    let table = dialect_ffi::cflag_table();
    let mut out = String::new();
    out.push('[');
    for (i, entry) in table.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"name\":\"");
        json_escape(&mut out, &entry.suffix);
        out.push_str("\",\"minVersion\":");
        out.push_str(&entry.min_version.to_string());
        out.push_str(",\"category\":\"");
        json_escape(&mut out, &entry.category);
        out.push_str("\"}");
    }
    out.push(']');
    set_result(&out);
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
    let funcs = lsp.host.available_functions();

    // Deduplicate by name (multiple arities collapse to one entry).
    let mut seen = HashSet::new();
    let items: Vec<AvailableFunction> = funcs
        .iter()
        .filter(|f| seen.insert(f.name.clone()))
        .map(|f| AvailableFunction {
            name: f.name.clone(),
        })
        .collect();
    let count = items.len() as i32;
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
    let parse_diags: Vec<_> = lsp.host.diagnostics(WASM_DOC_URI).to_vec();

    // Run semantic validation (function name/arity, table/column checks).
    let validation_config = syntaqlite::validation::ValidationConfig::default();
    let validation_diags = lsp.host.validate(WASM_DOC_URI, &validation_config);

    let total_count = parse_diags.len() + validation_diags.len();

    // Serialize diagnostics as JSON array (no serde in WASM).
    let mut out = String::new();
    out.push('[');
    let mut first = true;
    for d in parse_diags.iter().chain(validation_diags.iter()) {
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str("{\"startOffset\":");
        out.push_str(&d.start_offset.to_string());
        out.push_str(",\"endOffset\":");
        out.push_str(&d.end_offset.to_string());
        out.push_str(",\"message\":\"");
        json_escape(&mut out, &d.message);
        out.push_str("\",\"severity\":\"");
        out.push_str(match d.severity {
            syntaqlite::lsp::Severity::Error => "error",
            syntaqlite::lsp::Severity::Warning => "warning",
            syntaqlite::lsp::Severity::Info => "info",
            syntaqlite::lsp::Severity::Hint => "hint",
        });
        out.push_str("\"}");
    }
    out.push(']');
    set_result(&out);

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

fn is_keyword_symbol(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

#[derive(Serialize)]
struct CompletionItem<'a> {
    label: &'a str,
    kind: &'a str,
}

fn run_completions(ptr: u32, len: u32, offset: u32, version: u32) -> i32 {
    let dialect_ptr = DIALECT_PTR.with(|p| p.get());
    if dialect_ptr == 0 {
        set_result("dialect pointer is not set; call wasm_set_dialect first");
        return -1;
    }
    let dialect = match resolve_dialect() {
        Ok(d) => d,
        Err(e) => {
            set_result(&e);
            return -1;
        }
    };
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
    let info = lsp
        .host
        .completion_info_at_offset(WASM_DOC_URI, offset as usize);
    let expected_set: HashSet<u32> = info.tokens.into_iter().collect();

    let mut seen = HashSet::new();
    let mut items: Vec<CompletionItem> = Vec::new();

    let mut expects_identifier = false;
    for &tok in &expected_set {
        if dialect.token_category(tok) == syntaqlite::dialect::TokenCategory::Identifier {
            expects_identifier = true;
            break;
        }
    }

    for i in 0..dialect.keyword_count() {
        let Some((code, name)) = dialect.keyword_entry(i) else {
            continue;
        };
        if !expected_set.contains(&code) || !is_keyword_symbol(name) {
            continue;
        }
        if seen.insert(name.to_string()) {
            items.push(CompletionItem {
                label: name,
                kind: "keyword",
            });
        }
    }

    // Only show functions in Expression or Unknown context — not in TableRef.
    let show_functions = expects_identifier
        && matches!(
            info.context,
            syntaqlite::lsp::CompletionContext::Expression
                | syntaqlite::lsp::CompletionContext::Unknown
        );

    // When identifiers are expected in an expression context, include built-in functions.
    // Collect owned names first since available_functions() returns owned Strings.
    let func_names: Vec<String> = if show_functions {
        lsp.host
            .available_functions()
            .into_iter()
            .filter(|f| seen.insert(f.name.clone()))
            .map(|f| f.name)
            .collect()
    } else {
        Vec::new()
    };
    for name in &func_names {
        items.push(CompletionItem {
            label: name,
            kind: "function",
        });
    }

    let count = items.len() as i32;
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

fn main() {}

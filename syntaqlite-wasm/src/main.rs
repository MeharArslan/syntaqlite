// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::{Cell, RefCell};
use std::slice;

use syntaqlite_runtime::dialect::ffi as dialect_ffi;
use syntaqlite_runtime::fmt::{FormatConfig, Formatter, KeywordCase};
use syntaqlite_runtime::{Dialect, Parser};

thread_local! {
    static RESULT_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static DIALECT_PTR: Cell<u32> = const { Cell::new(0) };
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
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_clear_dialect() {
    DIALECT_PTR.with(|p| p.set(0));
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

fn main() {}

//! Converts a C `SyntaqliteDialect` struct into a Rust `DialectInfo`.
//!
//! This module provides `dialect_to_info` which is used both by compiled-in
//! dialects (like `syntaqlite` crate) and by dynamically loaded `.so` dialects.

use std::ffi::CStr;

use crate::parser::nodes::{FieldDescriptor, FieldKind};
use crate::parser::Dialect;
use crate::DialectInfo;

// ── C ABI mirror structs ────────────────────────────────────────────────

pub const FIELD_NODE_ID: u8 = 0;
pub const FIELD_SPAN: u8 = 1;
pub const FIELD_BOOL: u8 = 2;
pub const FIELD_FLAGS: u8 = 3;
pub const FIELD_ENUM: u8 = 4;

#[repr(C)]
pub struct RawFieldMeta {
    pub offset: u16,
    pub kind: u8,
    pub name: *const std::ffi::c_char,
    pub display: *const *const std::ffi::c_char,
    pub display_count: u8,
}

/// Mirrors the C `SyntaqliteDialect` struct defined in `include/syntaqlite/dialect.h`.
///
/// The parser vtable fields are inline (not behind a separate pointer).
#[repr(C)]
pub struct RawSyntaqliteDialect {
    pub name: *const std::ffi::c_char,

    // Parser vtable (Lemon lifecycle)
    pub lemon_alloc: *const std::ffi::c_void,
    pub lemon_init: *const std::ffi::c_void,
    pub lemon_finalize: *const std::ffi::c_void,
    pub lemon_free: *const std::ffi::c_void,
    pub lemon_parse: *const std::ffi::c_void,
    pub lemon_trace: *const std::ffi::c_void,

    // Range metadata
    pub range_meta: *const std::ffi::c_void,

    // Well-known token IDs
    pub tk_space: i32,
    pub tk_semi: i32,
    pub tk_comment: i32,

    // AST metadata
    pub node_count: u32,
    pub node_names: *const *const std::ffi::c_char,
    pub field_meta: *const *const RawFieldMeta,
    pub field_meta_counts: *const u8,
    pub list_tags: *const u8,

    // Formatter bytecode
    pub fmt_data: *const u8,
    pub fmt_data_len: u32,
}

// ── Conversion ──────────────────────────────────────────────────────────

/// Convert a C `SyntaqliteDialect` into a Rust `DialectInfo`.
///
/// The `SyntaqliteDialect` struct contains both the parser vtable and AST
/// metadata inline — no separate pointer indirection needed.
///
/// # Safety
/// All pointers must be valid. Arrays must have lengths matching `node_count`.
pub unsafe fn dialect_to_info(raw: *const RawSyntaqliteDialect) -> DialectInfo {
    let raw = unsafe { &*raw };
    let node_count = raw.node_count as usize;

    // Convert node names
    let c_names = unsafe { std::slice::from_raw_parts(raw.node_names, node_count) };
    let mut names: Vec<&'static str> = Vec::with_capacity(node_count);
    for &name_ptr in c_names {
        let s = c_str_to_static(name_ptr);
        names.push(s);
    }
    let node_names: &'static [&'static str] = Box::leak(names.into_boxed_slice());

    // Convert field metadata
    let c_meta_ptrs = unsafe { std::slice::from_raw_parts(raw.field_meta, node_count) };
    let c_meta_counts = unsafe { std::slice::from_raw_parts(raw.field_meta_counts, node_count) };

    let mut descriptors: Vec<&'static [FieldDescriptor]> = Vec::with_capacity(node_count);
    for i in 0..node_count {
        let count = c_meta_counts[i] as usize;
        if count == 0 || c_meta_ptrs[i].is_null() {
            descriptors.push(&[]);
            continue;
        }
        let c_fields = unsafe { std::slice::from_raw_parts(c_meta_ptrs[i], count) };
        let mut fields: Vec<FieldDescriptor> = Vec::with_capacity(count);
        for cf in c_fields {
            let name = c_str_to_static(cf.name);
            let kind = convert_field_kind(cf);
            fields.push(FieldDescriptor {
                offset: cf.offset,
                kind,
                name,
            });
        }
        descriptors.push(Box::leak(fields.into_boxed_slice()));
    }
    let field_descriptors: &'static [&'static [FieldDescriptor]] =
        Box::leak(descriptors.into_boxed_slice());

    // Build is_list from list_tags array
    let c_list_tags = unsafe { std::slice::from_raw_parts(raw.list_tags, node_count) };
    let list_tags: &'static [u8] = Box::leak(c_list_tags.to_vec().into_boxed_slice());
    // Store the leaked slice in a static so the fn pointer can reference it.
    // This works because dialect_to_info is called once per dialect in a LazyLock.
    IS_LIST_TABLE.store(
        list_tags.as_ptr() as *mut u8,
        std::sync::atomic::Ordering::Release,
    );
    IS_LIST_TABLE_LEN.store(node_count, std::sync::atomic::Ordering::Release);

    // The dialect pointer IS the raw pointer itself — the parser vtable is inline.
    let dialect = unsafe { Dialect::from_raw(raw as *const RawSyntaqliteDialect as *const std::ffi::c_void) };

    #[cfg(feature = "fmt")]
    let fmt = if !raw.fmt_data.is_null() && raw.fmt_data_len > 0 {
        let data = unsafe { std::slice::from_raw_parts(raw.fmt_data, raw.fmt_data_len as usize) };
        crate::fmt::LoadedFmt::load(data)
            .expect("failed to load fmt bytecode from C dialect")
            .into_static()
    } else {
        panic!("C dialect has no fmt data but fmt feature is enabled")
    };

    DialectInfo {
        dialect,
        field_descriptors,
        node_names,
        is_list: is_list_from_table,
        #[cfg(feature = "fmt")]
        fmt,
    }
}

// Global storage for the is_list lookup table.
// Safe because dialect_to_info is called once per dialect in a LazyLock.
static IS_LIST_TABLE: std::sync::atomic::AtomicPtr<u8> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());
static IS_LIST_TABLE_LEN: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

fn is_list_from_table(tag: u32) -> bool {
    let ptr = IS_LIST_TABLE.load(std::sync::atomic::Ordering::Acquire);
    let len = IS_LIST_TABLE_LEN.load(std::sync::atomic::Ordering::Acquire);
    if ptr.is_null() || (tag as usize) >= len {
        return false;
    }
    unsafe { *ptr.add(tag as usize) != 0 }
}

/// Convert a C field's kind + display arrays into a Rust `FieldKind`.
fn convert_field_kind(cf: &RawFieldMeta) -> FieldKind {
    match cf.kind {
        FIELD_NODE_ID => FieldKind::NodeId,
        FIELD_SPAN => FieldKind::Span,
        FIELD_BOOL => FieldKind::Bool,
        FIELD_FLAGS | FIELD_ENUM => {
            let count = cf.display_count as usize;
            let display: &'static [&'static str] = if count > 0 && !cf.display.is_null() {
                let c_ptrs = unsafe { std::slice::from_raw_parts(cf.display, count) };
                let strs: Vec<&'static str> = c_ptrs.iter().map(|&p| c_str_to_static(p)).collect();
                Box::leak(strs.into_boxed_slice())
            } else {
                &[]
            };
            if cf.kind == FIELD_FLAGS {
                FieldKind::Flags(display)
            } else {
                FieldKind::Enum(display)
            }
        }
        _ => panic!("unknown C field kind: {}", cf.kind),
    }
}

/// Convert a C string pointer to a leaked `&'static str`.
fn c_str_to_static(ptr: *const std::ffi::c_char) -> &'static str {
    let cstr = unsafe { CStr::from_ptr(ptr) };
    let s: &str = cstr.to_str().expect("non-UTF8 string from C dialect");
    Box::leak(s.to_string().into_boxed_str())
}

// ── Dynamic loading ─────────────────────────────────────────────────────

pub fn load_dialect(path: &std::path::Path) -> Result<&'static DialectInfo, String> {
    use libloading::{Library, Symbol};

    let lib = unsafe { Library::new(path) }
        .map_err(|e| format!("failed to load dialect library {:?}: {}", path, e))?;

    // Convention: the shared library exports `syntaqlite_dialect` returning
    // a *const RawSyntaqliteDialect.
    let func: Symbol<unsafe extern "C" fn() -> *const RawSyntaqliteDialect> =
        unsafe { lib.get(b"syntaqlite_dialect") }
            .map_err(|e| format!("symbol `syntaqlite_dialect` not found: {}", e))?;

    let raw = unsafe { func() };
    if raw.is_null() {
        return Err("syntaqlite_dialect() returned null".into());
    }

    let info = unsafe { dialect_to_info(raw) };
    let info = Box::leak(Box::new(info));

    // Leak the library handle to keep the .so loaded
    std::mem::forget(lib);

    Ok(info)
}

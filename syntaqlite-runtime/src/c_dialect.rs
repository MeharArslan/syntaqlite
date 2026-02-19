//! Converts C `SyntaqliteDialect` structs into fully-owned Rust types.
//!
//! A single `convert()` call returns one `ConvertedDialect`. No leaking.

use std::ffi::CStr;

use crate::parser::nodes::{dump_node_with, FieldDescriptor, FieldKind, NodeId};
use crate::parser::{Dialect, Session};

#[cfg(feature = "fmt")]
use crate::fmt::{LoadedFmt, NodeInfo};

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

    // Parse tables + reduce actions
    pub tables: *const std::ffi::c_void,
    pub reduce_actions: *const std::ffi::c_void,

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

// ── ConvertedDialect ────────────────────────────────────────────────────

/// Fully-owned Rust representation of a C dialect. No leaked memory.
pub struct ConvertedDialect {
    pub dialect: Dialect,
    pub node_names: Vec<String>,
    pub field_descriptors: Vec<Vec<FieldDescriptor>>,
    #[cfg(feature = "fmt")]
    pub node_info: NodeInfo,
    #[cfg(feature = "fmt")]
    pub fmt: LoadedFmt,
}

unsafe impl Send for ConvertedDialect {}
unsafe impl Sync for ConvertedDialect {}

impl ConvertedDialect {
    pub fn parser(&self) -> crate::parser::Parser {
        crate::parser::Parser::new(&self.dialect)
    }

    pub fn dump_node(
        &self,
        session: &Session<'_>,
        id: NodeId,
        out: &mut String,
        indent: usize,
    ) {
        dump_node_with(
            &|nid| session.node_ptr(nid),
            session.source(),
            &self.field_descriptors,
            &self.node_names,
            id,
            out,
            indent,
        )
    }
}

/// Single entry point: converts a C dialect into fully owned Rust data.
/// No leaking.
///
/// # Safety
/// The pointer must be valid. All C arrays must have lengths matching `node_count`.
pub unsafe fn convert(raw: *const RawSyntaqliteDialect) -> ConvertedDialect {
    let raw_ref = unsafe { &*raw };
    let node_count = raw_ref.node_count as usize;

    // Dialect handle
    let dialect = unsafe { Dialect::from_raw(raw as *const std::ffi::c_void) };

    // Node names
    let c_names = unsafe { std::slice::from_raw_parts(raw_ref.node_names, node_count) };
    let node_names: Vec<String> = c_names.iter().map(|&p| c_str_to_owned(p)).collect();

    // Field descriptors
    let c_meta_ptrs = unsafe { std::slice::from_raw_parts(raw_ref.field_meta, node_count) };
    let c_meta_counts = unsafe { std::slice::from_raw_parts(raw_ref.field_meta_counts, node_count) };
    let mut field_descriptors: Vec<Vec<FieldDescriptor>> = Vec::with_capacity(node_count);
    for i in 0..node_count {
        let count = c_meta_counts[i] as usize;
        if count == 0 || c_meta_ptrs[i].is_null() {
            field_descriptors.push(Vec::new());
            continue;
        }
        let c_fields = unsafe { std::slice::from_raw_parts(c_meta_ptrs[i], count) };
        let mut fields: Vec<FieldDescriptor> = Vec::with_capacity(count);
        for cf in c_fields {
            let name = c_str_to_owned(cf.name);
            let kind = convert_field_kind(cf);
            fields.push(FieldDescriptor {
                offset: cf.offset,
                kind,
                name,
            });
        }
        field_descriptors.push(fields);
    }

    // Formatter-specific data
    #[cfg(feature = "fmt")]
    let node_info = {
        let c_list_tags = unsafe { std::slice::from_raw_parts(raw_ref.list_tags, node_count) };
        let list_tags: Vec<bool> = c_list_tags.iter().map(|&b| b != 0).collect();
        NodeInfo {
            field_descriptors: field_descriptors.clone(),
            list_tags,
        }
    };

    #[cfg(feature = "fmt")]
    let fmt = {
        if raw_ref.fmt_data.is_null() || raw_ref.fmt_data_len == 0 {
            panic!("C dialect has no fmt data");
        }
        let data =
            unsafe { std::slice::from_raw_parts(raw_ref.fmt_data, raw_ref.fmt_data_len as usize) };
        LoadedFmt::load(data).expect("failed to load fmt bytecode from C dialect")
    };

    ConvertedDialect {
        dialect,
        node_names,
        field_descriptors,
        #[cfg(feature = "fmt")]
        node_info,
        #[cfg(feature = "fmt")]
        fmt,
    }
}

// ── Internal helpers ────────────────────────────────────────────────────

/// Convert a C field's kind + display arrays into a Rust `FieldKind`.
fn convert_field_kind(cf: &RawFieldMeta) -> FieldKind {
    match cf.kind {
        FIELD_NODE_ID => FieldKind::NodeId,
        FIELD_SPAN => FieldKind::Span,
        FIELD_BOOL => FieldKind::Bool,
        FIELD_FLAGS | FIELD_ENUM => {
            let count = cf.display_count as usize;
            let display: Vec<String> = if count > 0 && !cf.display.is_null() {
                let c_ptrs = unsafe { std::slice::from_raw_parts(cf.display, count) };
                c_ptrs.iter().map(|&p| c_str_to_owned(p)).collect()
            } else {
                Vec::new()
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

/// Convert a C string pointer to an owned `String`.
fn c_str_to_owned(ptr: *const std::ffi::c_char) -> String {
    let cstr = unsafe { CStr::from_ptr(ptr) };
    cstr.to_str().expect("non-UTF8 string from C dialect").to_string()
}

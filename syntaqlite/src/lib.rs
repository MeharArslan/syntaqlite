// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

pub mod ast_traits;
pub mod parser;

pub use parser::{NodeId, NodeReader, ParseError, Parser, StatementCursor};

#[cfg(feature = "fmt")]
pub mod fmt;

pub mod catalog;
pub mod dialect;

pub use dialect::Dialect;

#[cfg(feature = "validation")]
pub mod validation;

#[cfg(feature = "lsp")]
pub mod lsp;

#[cfg(feature = "sqlite")]
pub mod sqlite;

// ── Shared field extraction ────────────────────────────────────────────

use dialect::ffi::{FIELD_BOOL, FIELD_ENUM, FIELD_FLAGS, FIELD_NODE_ID, FIELD_SPAN, FieldMeta};
use parser::nodes::{FieldVal, SourceSpan};

/// Extract a single field value from a raw node pointer using field metadata.
///
/// # Safety
/// `ptr` must point to a valid node struct whose field at `m.offset` has
/// the type indicated by `m.kind`.
pub(crate) unsafe fn extract_field_val<'a>(
    ptr: *const u8,
    m: &FieldMeta,
    source: &'a str,
) -> FieldVal<'a> {
    unsafe {
        let field_ptr = ptr.add(m.offset as usize);
        match m.kind {
            FIELD_NODE_ID => FieldVal::NodeId(NodeId(*(field_ptr as *const u32))),
            FIELD_SPAN => {
                let span = &*(field_ptr as *const SourceSpan);
                if span.length == 0 {
                    FieldVal::Span("", 0)
                } else {
                    FieldVal::Span(span.as_str(source), span.offset)
                }
            }
            FIELD_BOOL => FieldVal::Bool(*(field_ptr as *const u32) != 0),
            FIELD_FLAGS => FieldVal::Flags(*field_ptr),
            FIELD_ENUM => FieldVal::Enum(*(field_ptr as *const u32)),
            _ => panic!("unknown C field kind: {}", m.kind),
        }
    }
}

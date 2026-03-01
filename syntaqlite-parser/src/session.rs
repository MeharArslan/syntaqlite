// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::ffi::CStr;

use crate::dialect::Dialect;
use crate::nodes::{ArenaNode, NodeId, NodeList};
use crate::parser as ffi;
use crate::dialect_traits::DialectNodeType;

/// A source span describing where an error node was recorded in the arena.
///
/// Returned by [`RawNodeReader::required_node`] and [`RawNodeReader::optional_node`]
/// when the resolved arena node is an `ErrorNode` (tag 0) rather than the
/// expected typed node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorSpan {
    /// Byte offset of the error token in the source text.
    pub offset: u32,
    /// Byte length of the error token (0 = unknown).
    pub length: u32,
}

/// A parse error with a human-readable message and optional source location.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    /// Byte offset of the error token in the source text.
    /// `None` if the error location is unknown.
    pub offset: Option<usize>,
    /// Byte length of the error token.
    /// `None` if the error length is unknown.
    pub length: Option<usize>,
    /// Root node of a partially recovered parse tree, if error recovery
    /// succeeded. The tree may contain `ErrorNode` placeholders (tag 0)
    /// in positions where the parser recovered (e.g. interpolation holes).
    /// `None` when the error was unrecoverable and no tree was produced.
    pub root: Option<NodeId>,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

// ── NodeReader ──────────────────────────────────────────────────────────────

/// A lightweight, `Copy` handle for reading nodes from the parser arena.
///
/// This is the read-only half of a cursor state. Dialect crates embed it in
/// view structs so that accessor methods can resolve `NodeId` children
/// without requiring a back-reference to the full cursor.
///
/// # Safety invariant
/// The raw pointer must remain valid for `'a`. This is guaranteed when
/// `RawNodeReader` is obtained from a cursor state (which borrows the parser
/// exclusively for `'a`).
#[derive(Clone, Copy)]
pub struct RawNodeReader<'a> {
    pub(crate) raw: *mut ffi::Parser,
    pub(crate) source: &'a str,
}

impl<'a> RawNodeReader<'a> {
    /// Construct a `RawNodeReader` from a raw parser pointer and source text.
    ///
    /// # Safety
    /// `raw` must be a valid, non-null parser pointer that remains valid
    /// for the lifetime `'a`.
    pub unsafe fn new(raw: *mut ffi::Parser, source: &'a str) -> Self {
        RawNodeReader { raw, source }
    }

    /// Enumerate all child NodeIds of a node using dialect metadata.
    ///
    /// For regular nodes, returns all `Index`-typed (child node) fields.
    /// For list nodes, returns the list's children.
    /// Null child IDs are omitted from the result.
    pub fn child_node_ids(&self, id: NodeId, dialect: &Dialect) -> Vec<NodeId> {
        let Some((ptr, tag)) = self.node_ptr(id) else {
            return vec![];
        };

        if dialect.is_list(tag) {
            // SAFETY: ptr is valid and tag confirms list layout.
            let list = unsafe { &*(ptr as *const NodeList) };
            return list
                .children()
                .iter()
                .copied()
                .filter(|id| !id.is_null())
                .collect();
        }

        let meta = dialect.field_meta(tag);
        let mut children = Vec::new();
        for field in meta {
            if field.kind == crate::dialect::FIELD_NODE_ID {
                // SAFETY: ptr is a valid arena pointer, field.offset is a
                // codegen-computed offset within the node struct, and the
                // field at that offset is a u32 (raw NodeId).
                let child_raw = unsafe {
                    let field_ptr = ptr.add(field.offset as usize) as *const u32;
                    *field_ptr
                };
                let child_id = NodeId(child_raw);
                if !child_id.is_null() {
                    children.push(child_id);
                }
            }
        }
        children
    }

    /// Resolve a `NodeId` to a typed reference, validating the tag matches.
    /// Returns `None` if null, invalid, or tag mismatch.
    pub fn resolve_as<T: ArenaNode>(&self, id: NodeId) -> Option<&'a T> {
        let (ptr, tag) = self.node_ptr(id)?;
        if tag != T::TAG {
            return None;
        }
        // SAFETY: tag matches T::TAG, confirming the arena node has type T.
        // ptr is valid for 'a (guaranteed by NodeReader's construction from a
        // live parser). T is #[repr(C)] with a u32 tag as its first field,
        // matching the arena layout.
        Some(unsafe { &*(ptr as *const T) })
    }

    /// Resolve a `NodeId` as a `NodeList` (for list nodes).
    /// Returns `None` if null or invalid.
    pub fn resolve_list(&self, id: NodeId) -> Option<&'a NodeList> {
        let (ptr, _) = self.node_ptr(id)?;
        // SAFETY: ptr is valid for 'a. List nodes have NodeList layout
        // (tag, count, children[count]). The caller is responsible for
        // ensuring the id refers to a list node (enforced by codegen).
        Some(unsafe { &*(ptr as *const NodeList) })
    }

    /// Get a raw pointer to a node in the arena. Returns `(pointer, tag)`.
    pub fn node_ptr(&self, id: NodeId) -> Option<(*const u8, u32)> {
        if id.is_null() {
            return None;
        }
        // SAFETY: self.raw is valid for 'a (guaranteed by NodeReader's construction
        // from a live parser). The returned pointer is null-checked; all arena nodes
        // start with a u32 tag, so the dereference is valid and aligned.
        unsafe {
            let ptr = ffi::syntaqlite_parser_node(self.raw, id.0);
            if ptr.is_null() {
                return None;
            }
            let tag = *ptr;
            Some((ptr as *const u8, tag))
        }
    }

    /// Return the node tag for the given ID, or `None` if null/invalid.
    pub fn node_tag(&self, id: NodeId) -> Option<u32> {
        self.node_ptr(id).map(|(_, tag)| tag)
    }

    /// Resolve a required node field: panics (in debug) if `id` is null,
    /// returns `Err(ErrorSpan)` if the arena node is an error placeholder,
    /// or `Err(ErrorSpan { 0, 0 })` if the type tag mismatches.
    pub fn required_node<T: DialectNodeType<'a>>(&self, id: NodeId) -> Result<T, ErrorSpan> {
        debug_assert!(!id.is_null(), "required field has null NodeId");
        self.resolve_or_error(id)
    }

    /// Resolve an optional node field: returns `Ok(None)` if `id` is null,
    /// `Err(ErrorSpan)` if the arena node is an error placeholder, or
    /// `Ok(Some(T))` on success.
    pub fn optional_node<T: DialectNodeType<'a>>(&self, id: NodeId) -> Result<Option<T>, ErrorSpan> {
        if id.is_null() {
            return Ok(None);
        }
        self.resolve_or_error(id).map(Some)
    }

    fn resolve_or_error<T: DialectNodeType<'a>>(&self, id: NodeId) -> Result<T, ErrorSpan> {
        let Some((ptr, tag)) = self.node_ptr(id) else {
            return Err(ErrorSpan {
                offset: 0,
                length: 0,
            });
        };
        if tag == ffi::SYNTAQLITE_ERROR_NODE_TAG {
            // SAFETY: tag 0 guarantees SyntaqliteErrorNode layout ({ u32, u32, u32 }, 12 bytes).
            let e = unsafe { &*(ptr as *const ffi::ErrorNode) };
            return Err(ErrorSpan {
                offset: e.offset,
                length: e.length,
            });
        }
        // SAFETY: NodeReader<'a> is Copy and all its data (raw pointer, source
        // reference) is valid for 'a. Re-casting &self to &'a NodeReader<'a>
        // extends the borrow lifetime to 'a, which is safe because the
        // underlying parser arena lives for 'a (same pattern as resolve_as).
        let reader: &'a RawNodeReader<'a> = unsafe { &*(self as *const RawNodeReader<'a>) };
        T::from_arena(reader, id).ok_or(ErrorSpan {
            offset: 0,
            length: 0,
        })
    }

    /// Extract typed field values from a node, using dialect metadata.
    ///
    /// Returns `(tag, fields)` where `tag` is the node's type tag and
    /// `fields` contains the extracted field values. Returns `None` if
    /// the node ID is null or invalid.
    pub fn extract_fields(
        &self,
        id: NodeId,
        dialect: &Dialect,
    ) -> Option<(u32, crate::nodes::Fields<'a>)> {
        let (ptr, tag) = self.node_ptr(id)?;
        // SAFETY: ptr is a valid arena pointer from node_ptr(); tag matches
        // the node's type, so dialect.field_meta(tag) is correct.
        let fields = unsafe { crate::dialect::extract_fields(dialect, ptr, tag, self.source) };
        Some((tag, fields))
    }

    /// The source text bound to this reader.
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// Return all non-whitespace, non-comment token positions captured
    /// during parsing. Requires `collect_tokens: true` in `ParserConfig`.
    pub fn tokens(&self) -> &[ffi::TokenPos] {
        // SAFETY: raw is valid; syntaqlite_parser_tokens returns a pointer valid
        // for the lifetime of &self (until the next reset/destroy, which need &mut).
        unsafe { ffi_slice(self.raw, ffi::syntaqlite_parser_tokens) }
    }

    /// Return all macro regions captured during parsing.
    pub fn macro_regions(&self) -> &[ffi::MacroRegion] {
        // SAFETY: raw is valid; syntaqlite_parser_macro_regions returns a pointer
        // valid for the lifetime of &self.
        unsafe { ffi_slice(self.raw, ffi::syntaqlite_parser_macro_regions) }
    }

    /// Access the raw C parser pointer.
    pub fn raw(&self) -> *mut ffi::Parser {
        self.raw
    }

    /// If `id` refers to a list node (per the dialect), return its child node IDs.
    pub fn list_children(&self, id: NodeId, dialect: &Dialect) -> Option<&'a [NodeId]> {
        let (ptr, tag) = self.node_ptr(id)?;
        if !dialect.is_list(tag) {
            return None;
        }
        // SAFETY: ptr is a valid arena pointer and the tag confirms it is a
        // list node, so it has NodeList layout (tag, count, children[count]).
        Some(unsafe { &*(ptr as *const NodeList) }.children())
    }

    /// Dump an AST node tree as indented text. Uses C-side metadata (field
    /// names, display strings) so no Rust-side string tables are needed.
    pub fn dump_node(&self, id: NodeId, out: &mut String, indent: usize) {
        unsafe extern "C" {
            fn free(ptr: *mut std::ffi::c_void);
        }
        // SAFETY: raw is valid; dump_node returns a malloc'd NUL-terminated
        // string (or null). We free the C string after copying.
        unsafe {
            let ptr = ffi::syntaqlite_dump_node(self.raw, id.0, indent as u32);
            if !ptr.is_null() {
                let cstr = CStr::from_ptr(ptr);
                out.push_str(&cstr.to_string_lossy());
                free(ptr as *mut std::ffi::c_void);
            }
        }
    }
}

/// Build a slice from an FFI function that returns a pointer and writes a count.
///
/// # Safety
/// `raw` must be a valid parser pointer. `f` must return a pointer that is valid
/// for the caller's borrow of the parser, and write the element count into the
/// provided `*mut u32`.
unsafe fn ffi_slice<'a, T>(
    raw: *mut ffi::Parser,
    f: unsafe extern "C" fn(*mut ffi::Parser, *mut u32) -> *const T,
) -> &'a [T] {
    let mut count: u32 = 0;
    let ptr = unsafe { f(raw, &mut count) };
    if count == 0 || ptr.is_null() {
        return &[];
    }
    unsafe { std::slice::from_raw_parts(ptr, count as usize) }
}

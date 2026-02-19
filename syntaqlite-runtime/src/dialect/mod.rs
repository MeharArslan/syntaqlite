//! Dialect types: the opaque handle, C ABI mirror structs, and typed access traits.

pub mod ffi;

use crate::parser::nodes::NodeId;
use crate::parser::{FieldVal, Fields, NodeList, ParseError, Session, SourceSpan};

use ffi::{FieldMeta, FIELD_BOOL, FIELD_ENUM, FIELD_FLAGS, FIELD_NODE_ID, FIELD_SPAN};

// ── Opaque dialect handle ──────────────────────────────────────────────

/// An opaque dialect handle. Dialect crates (e.g. `syntaqlite`) provide a
/// function that returns a `&'static Dialect<'static>` for their grammar.
#[derive(Clone, Copy)]
pub struct Dialect<'d> {
    pub(crate) raw: &'d ffi::Dialect,
}

impl<'d> Dialect<'d> {
    /// Create a `Dialect` from a raw C pointer returned by a dialect's
    /// FFI function (e.g. `syntaqlite_sqlite_dialect`).
    ///
    /// # Safety
    /// The pointer must point to a valid `ffi::Dialect` whose data lives
    /// at least as long as `'d`.
    pub unsafe fn from_raw(raw: *const ffi::Dialect) -> Self {
        unsafe { Dialect { raw: &*raw } }
    }

    /// Whether the given node tag represents a list node.
    pub(crate) fn is_list(&self, tag: u32) -> bool {
        let idx = tag as usize;
        if idx >= self.raw.node_count as usize {
            return false;
        }
        unsafe { *self.raw.list_tags.add(idx) != 0 }
    }

    /// Return the field metadata for a node tag.
    pub(crate) fn field_meta(&self, tag: u32) -> &[FieldMeta] {
        let idx = tag as usize;
        if idx >= self.raw.node_count as usize {
            return &[];
        }
        let count = unsafe { *self.raw.field_meta_counts.add(idx) } as usize;
        let ptr = unsafe { *self.raw.field_meta.add(idx) };
        if count == 0 || ptr.is_null() {
            return &[];
        }
        unsafe { std::slice::from_raw_parts(ptr, count) }
    }

    /// Extract typed field values from a raw node pointer.
    pub(crate) fn extract_fields<'a>(
        &self,
        ptr: *const u8,
        tag: u32,
        source: &'a str,
    ) -> Fields<'a> {
        let meta = self.field_meta(tag);
        let mut fields = Fields::new();
        for m in meta {
            fields.push(unsafe { extract_one(ptr, m, source) });
        }
        fields
    }

    /// Look up a string from the C string table by index.
    /// The returned `&str` borrows from the dialect's underlying data.
    pub(crate) fn fmt_string(&self, sid: u16) -> &'d str {
        let idx = sid as usize;
        assert!(
            idx < self.raw.fmt_string_count as usize,
            "string index {} out of bounds (count={})",
            idx,
            self.raw.fmt_string_count,
        );
        unsafe {
            let cstr = std::ffi::CStr::from_ptr(*self.raw.fmt_strings.add(idx));
            cstr.to_str().expect("invalid UTF-8 in fmt string")
        }
    }

    /// Look up a value in the enum display table.
    pub(crate) fn fmt_enum_display_val(&self, idx: usize) -> u16 {
        assert!(
            idx < self.raw.fmt_enum_display_count as usize,
            "enum_display index {} out of bounds",
            idx,
        );
        unsafe { *self.raw.fmt_enum_display.add(idx) }
    }

    /// Look up the ops byte slice and op count for a given node tag.
    /// Returns `None` if the tag has no formatter ops.
    pub(crate) fn fmt_dispatch(&self, tag: u32) -> Option<(&[u8], usize)> {
        let idx = tag as usize;
        if idx >= self.raw.fmt_dispatch_count as usize {
            return None;
        }
        let packed = unsafe { *self.raw.fmt_dispatch.add(idx) };
        let offset = (packed >> 16) as u16;
        let length = (packed & 0xFFFF) as u16;
        if offset == 0xFFFF {
            return None;
        }
        let byte_offset = offset as usize * 6;
        let byte_len = length as usize * 6;
        let slice = unsafe {
            std::slice::from_raw_parts(self.raw.fmt_ops.add(byte_offset), byte_len)
        };
        Some((slice, length as usize))
    }

    /// Find the earliest source offset in a node's subtree.
    pub(crate) fn first_source_offset(
        &self,
        session: &Session<'_>,
        node_id: NodeId,
    ) -> Option<u32> {
        if node_id.is_null() {
            return None;
        }
        let (ptr, tag) = session.node_ptr(node_id)?;

        if self.is_list(tag) {
            let list = unsafe { &*(ptr as *const NodeList) };
            let children = list.children();
            if !children.is_empty() {
                return self.first_source_offset(session, children[0]);
            }
            return None;
        }

        let source = session.source();
        let fields = self.extract_fields(ptr, tag, source);

        let mut min: Option<u32> = None;
        for field in fields.iter() {
            if let FieldVal::Span(s, offset) = field {
                if !s.is_empty() {
                    min = Some(match min {
                        Some(prev) => prev.min(*offset),
                        None => *offset,
                    });
                }
            }
        }
        if min.is_some() {
            return min;
        }

        for field in fields.iter() {
            if let FieldVal::NodeId(child_id) = field {
                if let Some(off) = self.first_source_offset(session, *child_id) {
                    return Some(off);
                }
            }
        }

        None
    }

    /// Find the latest source offset (end) in a node's subtree.
    pub(crate) fn last_source_offset(
        &self,
        session: &Session<'_>,
        node_id: NodeId,
    ) -> Option<u32> {
        if node_id.is_null() {
            return None;
        }
        let (ptr, tag) = session.node_ptr(node_id)?;

        if self.is_list(tag) {
            let list = unsafe { &*(ptr as *const NodeList) };
            let children = list.children();
            if !children.is_empty() {
                return self.last_source_offset(session, children[children.len() - 1]);
            }
            return None;
        }

        let source = session.source();
        let fields = self.extract_fields(ptr, tag, source);

        let mut max: Option<u32> = None;
        for field in fields.iter() {
            if let FieldVal::Span(s, offset) = field {
                if !s.is_empty() {
                    let end = *offset + s.len() as u32;
                    max = Some(match max {
                        Some(prev) => prev.max(end),
                        None => end,
                    });
                }
            }
        }
        if max.is_some() {
            for field in fields.iter() {
                if let FieldVal::NodeId(child_id) = field {
                    if let Some(end) = self.last_source_offset(session, *child_id) {
                        max = Some(max.unwrap().max(end));
                    }
                }
            }
            return max;
        }

        for field in fields.iter() {
            if let FieldVal::NodeId(child_id) = field {
                if let Some(end) = self.last_source_offset(session, *child_id) {
                    max = Some(match max {
                        Some(prev) => prev.max(end),
                        None => end,
                    });
                }
            }
        }

        max
    }
}

// SAFETY: The dialect wraps a reference to a C struct with no mutable state.
// The raw pointers inside ffi::Dialect all point to immutable static data.
unsafe impl Send for Dialect<'_> {}
unsafe impl Sync for Dialect<'_> {}

// ── Field extraction ───────────────────────────────────────────────────

/// # Safety
/// `ptr` must point to a valid node struct whose field at `m.offset` has
/// the type indicated by `m.kind`.
unsafe fn extract_one<'a>(ptr: *const u8, m: &FieldMeta, source: &'a str) -> FieldVal<'a> {
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
            FIELD_FLAGS => FieldVal::Flags(*(field_ptr as *const u8)),
            FIELD_ENUM => FieldVal::Enum(*(field_ptr as *const u32)),
            _ => panic!("unknown C field kind: {}", m.kind),
        }
    }
}

// ── Typed access traits ────────────────────────────────────────────────

/// Trait that dialect crates implement to provide typed access to nodes and tokens.
///
/// The runtime provides blanket impls of `SessionExt` for any `D: DialectTypes`,
/// so dialect crates don't need to write any session-extension boilerplate.
pub trait DialectTypes: 'static {
    type Node<'a>;
    type TokenType: Copy + Into<u32>;

    /// # Safety
    /// The pointer must point to a valid node struct within the session's arena.
    unsafe fn node_from_raw<'a>(ptr: *const u32) -> Self::Node<'a>;
}

/// Extension trait adding typed node access and token feeding to `Session`.
///
/// Implemented via blanket impl for any `D: DialectTypes`.
pub trait SessionExt<'a, D: DialectTypes> {
    fn node(&self, id: NodeId) -> Option<D::Node<'a>>;
    fn feed(&mut self, token_type: D::TokenType, text: &str) -> Result<Option<NodeId>, ParseError>;
}

impl<'a, D: DialectTypes> SessionExt<'a, D> for Session<'a> {
    fn node(&self, id: NodeId) -> Option<D::Node<'a>> {
        let (ptr, _tag) = self.node_ptr(id)?;
        Some(unsafe { D::node_from_raw(ptr as *const u32) })
    }

    fn feed(&mut self, token_type: D::TokenType, text: &str) -> Result<Option<NodeId>, ParseError> {
        self.feed_token(token_type.into(), text)
    }
}

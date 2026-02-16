use std::fmt::Write;

use crate::generated::nodes::{FIELD_DESCRIPTORS, NODE_NAMES};
use crate::Session;

// ── Core types ──────────────────────────────────────────────────────────

pub const NULL_NODE: u32 = 0xFFFF_FFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct SourceSpan {
    pub offset: u32,
    pub length: u16,
}

impl SourceSpan {
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn as_str<'a>(&self, source: &'a str) -> &'a str {
        &source[self.offset as usize..(self.offset as usize + self.length as usize)]
    }
}

/// List node header — tag + count, followed by `count` child u32 IDs in
/// trailing data. The parser arena guarantees the trailing layout.
#[derive(Debug)]
#[repr(C)]
pub struct NodeList {
    pub tag: u32,
    pub count: u32,
}

impl NodeList {
    pub fn children(&self) -> &[u32] {
        // SAFETY: The arena allocates list nodes as { tag, count, children[count] }
        // contiguously, so `count` u32 values immediately follow this header.
        // NodeList is only constructed via Node::from_raw() which validates the
        // tag from a valid arena pointer.
        unsafe {
            let base = (self as *const NodeList).add(1) as *const u32;
            std::slice::from_raw_parts(base, self.count as usize)
        }
    }
}

// ── Field extraction ────────────────────────────────────────────────────

/// Extracted fields of a node, returned by value from `Node::fields()`.
#[derive(Debug)]
pub struct Fields<'a> {
    buf: [FieldVal<'a>; 16],
    len: usize,
}

impl<'a> Fields<'a> {
    pub(crate) fn new() -> Self {
        Self {
            buf: [FieldVal::NodeId(0); 16],
            len: 0,
        }
    }

    pub(crate) fn push(&mut self, val: FieldVal<'a>) {
        self.buf[self.len] = val;
        self.len += 1;
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl<'a> std::ops::Deref for Fields<'a> {
    type Target = [FieldVal<'a>];
    fn deref(&self) -> &[FieldVal<'a>] {
        &self.buf[..self.len]
    }
}

/// A typed field value extracted from a node struct.
#[derive(Clone, Copy, Debug)]
pub enum FieldVal<'a> {
    /// u32 node ID (child node or list reference).
    NodeId(u32),
    /// Source text from a SourceSpan field, with its source offset.
    Span(&'a str, u32),
    /// Boolean value.
    Bool(bool),
    /// Flags byte.
    Flags(u8),
    /// Enum ordinal.
    Enum(u32),
}

/// The kind of value stored in a node field.
#[derive(Clone, Copy, Debug)]
pub(crate) enum FieldKind {
    NodeId,
    Span,
    Bool,
    Flags(fn(u8) -> String),
    Enum(fn(u32) -> Option<&'static str>),
}

/// Metadata for one field of a node struct: its byte offset and value kind.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FieldDescriptor {
    pub offset: u16,
    pub kind: FieldKind,
    pub name: &'static str,
}

impl FieldDescriptor {
    /// Extract a `FieldVal` from a raw node pointer using this descriptor.
    ///
    /// # Safety
    /// `ptr` must point to a valid, well-aligned node struct whose field at
    /// `self.offset` has the type indicated by `self.kind`. `source` must be
    /// the original source string from which SourceSpan offsets were derived.
    pub(crate) unsafe fn extract<'a>(self, ptr: *const u8, source: &'a str) -> FieldVal<'a> {
        unsafe {
            let field_ptr = ptr.add(self.offset as usize);
            match self.kind {
                FieldKind::NodeId => FieldVal::NodeId(*(field_ptr as *const u32)),
                FieldKind::Span => {
                    let span = &*(field_ptr as *const SourceSpan);
                    if span.length == 0 {
                        FieldVal::Span("", 0)
                    } else {
                        FieldVal::Span(span.as_str(source), span.offset)
                    }
                }
                FieldKind::Bool => FieldVal::Bool(*(field_ptr as *const u32) != 0),
                FieldKind::Flags(_) => FieldVal::Flags(*(field_ptr as *const u8)),
                FieldKind::Enum(_) => FieldVal::Enum(*(field_ptr as *const u32)),
            }
        }
    }
}

// ── Dump ────────────────────────────────────────────────────────────────

/// Dump an AST node tree as indented text.
pub fn dump_node(session: &Session<'_>, id: u32, out: &mut String, indent: usize) {
    if id == NULL_NODE {
        return;
    }
    let Some(node) = session.node(id) else {
        return;
    };
    let source = session.source();
    let pad = "  ".repeat(indent);
    let tag = node.tag() as usize;

    if let Some(list) = node.as_list() {
        let _ = writeln!(out, "{pad}{} [{} items]", NODE_NAMES[tag], list.count);
        for &child_id in list.children() {
            dump_node(session, child_id, out, indent + 1);
        }
        return;
    }

    let _ = writeln!(out, "{pad}{}", NODE_NAMES[tag]);
    let descriptors = FIELD_DESCRIPTORS[tag];
    let fields = node.fields(source);

    for (desc, val) in descriptors.iter().zip(fields.iter()) {
        match (val, &desc.kind) {
            (FieldVal::NodeId(child_id), _) => {
                if *child_id == NULL_NODE {
                    let _ = writeln!(out, "{pad}  {}: (none)", desc.name);
                } else {
                    let _ = writeln!(out, "{pad}  {}:", desc.name);
                    dump_node(session, *child_id, out, indent + 2);
                }
            }
            (FieldVal::Span(text, _), _) => {
                if text.is_empty() {
                    let _ = writeln!(out, "{pad}  {}: null", desc.name);
                } else {
                    let _ = writeln!(out, "{pad}  {}: \"{text}\"", desc.name);
                }
            }
            (FieldVal::Bool(b), _) => {
                let s = if *b { "TRUE" } else { "FALSE" };
                let _ = writeln!(out, "{pad}  {}: {s}", desc.name);
            }
            (FieldVal::Flags(v), FieldKind::Flags(display)) => {
                let _ = writeln!(out, "{pad}  {}: {}", desc.name, display(*v));
            }
            (FieldVal::Enum(v), FieldKind::Enum(display)) => {
                let s = display(*v).unwrap_or("?");
                let _ = writeln!(out, "{pad}  {}: {s}", desc.name);
            }
            _ => {}
        }
    }
}

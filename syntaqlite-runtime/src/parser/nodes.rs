use std::fmt::Write;

// ── Core types ──────────────────────────────────────────────────────────

/// A typed wrapper around a raw arena node ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct NodeId(pub u32);

impl NodeId {
    /// Sentinel value representing a missing/null node.
    pub const NULL: NodeId = NodeId(0xFFFF_FFFF);

    /// Returns `true` if this is the null sentinel.
    pub fn is_null(&self) -> bool {
        self.0 == Self::NULL.0
    }
}

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
    pub fn children(&self) -> &[NodeId] {
        // SAFETY: The arena allocates list nodes as { tag, count, children[count] }
        // contiguously, so `count` u32 values immediately follow this header.
        // NodeList is only constructed via Node::from_raw() which validates the
        // tag from a valid arena pointer. NodeId is #[repr(transparent)] over u32,
        // so &[NodeId] has the same layout as &[u32].
        unsafe {
            let base = (self as *const NodeList).add(1) as *const NodeId;
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
    pub fn new() -> Self {
        Self {
            buf: [FieldVal::NodeId(NodeId::NULL); 16],
            len: 0,
        }
    }

    pub fn push(&mut self, val: FieldVal<'a>) {
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
    /// Node ID (child node or list reference).
    NodeId(NodeId),
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
///
/// For `Flags` and `Enum`, the display table holds human-readable names:
/// - `Flags`: indexed by bit position (0, 1, 2, …); each entry is the flag name
///   for that bit, or `""` if unused.
/// - `Enum`: indexed by ordinal (0, 1, 2, …); each entry is the variant name.
#[derive(Clone, Copy, Debug)]
pub enum FieldKind {
    NodeId,
    Span,
    Bool,
    Flags(&'static [&'static str]),
    Enum(&'static [&'static str]),
}

/// Metadata for one field of a node struct: its byte offset and value kind.
#[derive(Clone, Copy, Debug)]
pub struct FieldDescriptor {
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
    pub unsafe fn extract<'a>(self, ptr: *const u8, source: &'a str) -> FieldVal<'a> {
        unsafe {
            let field_ptr = ptr.add(self.offset as usize);
            match self.kind {
                FieldKind::NodeId => FieldVal::NodeId(NodeId(*(field_ptr as *const u32))),
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

/// Format a flags byte using the display table (indexed by bit position).
pub fn format_flags(value: u8, display: &[&str]) -> String {
    if value == 0 {
        return "(none)".into();
    }
    let mut s = String::new();
    for bit in 0..8u8 {
        if value & (1 << bit) != 0 {
            let name = display.get(bit as usize).copied().unwrap_or("?");
            if !name.is_empty() {
                if !s.is_empty() {
                    s.push(' ');
                }
                s.push_str(name);
            }
        }
    }
    if s.is_empty() { "(none)".into() } else { s }
}

// ── Dump ────────────────────────────────────────────────────────────────

/// Dump an AST node tree as indented text. Parameterized over dialect-specific
/// node metadata (field descriptors, node names) and a raw node pointer lookup.
pub fn dump_node_with(
    node_ptr_fn: &dyn Fn(NodeId) -> Option<(*const u8, u32)>,
    source: &str,
    field_descriptors: &[&[FieldDescriptor]],
    node_names: &[&str],
    id: NodeId,
    out: &mut String,
    indent: usize,
) {
    if id.is_null() {
        return;
    }
    let Some((ptr, tag)) = node_ptr_fn(id) else {
        return;
    };
    let tag_idx = tag as usize;
    let pad = "  ".repeat(indent);

    // Check if this is a list node (tag + count header)
    let descriptors = field_descriptors.get(tag_idx).copied().unwrap_or(&[]);
    if descriptors.is_empty() && tag != 0 {
        // Likely a list node — check by reading the NodeList header
        let list = unsafe { &*(ptr as *const NodeList) };
        let _ = writeln!(out, "{pad}{} [{} items]", node_names[tag_idx], list.count);
        for child_id in list.children() {
            dump_node_with(node_ptr_fn, source, field_descriptors, node_names, *child_id, out, indent + 1);
        }
        return;
    }

    let _ = writeln!(out, "{pad}{}", node_names[tag_idx]);

    let mut fields = Fields::new();
    for desc in descriptors {
        fields.push(unsafe { desc.extract(ptr, source) });
    }

    for (desc, val) in descriptors.iter().zip(fields.iter()) {
        match (val, &desc.kind) {
            (FieldVal::NodeId(child_id), _) => {
                if child_id.is_null() {
                    let _ = writeln!(out, "{pad}  {}: (none)", desc.name);
                } else {
                    let _ = writeln!(out, "{pad}  {}:", desc.name);
                    dump_node_with(node_ptr_fn, source, field_descriptors, node_names, *child_id, out, indent + 2);
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
                let s = format_flags(*v, display);
                let _ = writeln!(out, "{pad}  {}: {s}", desc.name);
            }
            (FieldVal::Enum(v), FieldKind::Enum(display)) => {
                let s = display.get(*v as usize).copied().unwrap_or("?");
                let _ = writeln!(out, "{pad}  {}: {s}", desc.name);
            }
            _ => {}
        }
    }
}

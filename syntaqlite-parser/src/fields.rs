use crate::nodes::SourceSpan;

/// A typed field value extracted from a node struct by generated code.
#[derive(Clone, Copy, Debug)]
pub enum FieldVal<'a> {
    /// u32 node ID (for `index` fields — child nodes and list references).
    NodeId(u32),
    /// Source text from a `SyntaqliteSourceSpan` field, with its source offset.
    Span(&'a str, u32),
    /// Boolean value (from `Bool` enum, repr(u32)).
    Bool(bool),
    /// Flags byte (from a flags union, repr(transparent) u8).
    Flags(u8),
    /// Enum ordinal (from a value enum, repr(u32)).
    Enum(u32),
}

/// The kind of value stored in a node field.
#[derive(Clone, Copy, Debug)]
pub(crate) enum FieldKind {
    NodeId,
    Span,
    Bool,
    Flags,
    Enum,
}

/// Metadata for one field of a node struct: its byte offset and value kind.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FieldDescriptor {
    pub offset: u16,
    pub kind: FieldKind,
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
                FieldKind::Flags => FieldVal::Flags(*(field_ptr as *const u8)),
                FieldKind::Enum => FieldVal::Enum(*(field_ptr as *const u32)),
            }
        }
    }
}

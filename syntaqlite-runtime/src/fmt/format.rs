use std::cell::Cell;

use crate::dialect::{
    Dialect, RawFieldMeta, RawSyntaqliteDialect, FIELD_BOOL, FIELD_ENUM, FIELD_FLAGS,
    FIELD_NODE_ID, FIELD_SPAN,
};
use crate::parser::{FieldVal, Fields, MacroRegion, NodeId, NodeList, Session, SourceSpan};

use super::bytecode::LoadedFmt;
use super::doc::{DocArena, DocId, NIL_DOC};
use super::interpret::{InterpretTrivia, interpret};
use super::trivia::TriviaCtx;

/// Zero-copy view over the C dialect's node metadata.
/// No allocations — reads directly from C static arrays.
pub struct NodeMeta {
    raw: *const RawSyntaqliteDialect,
}

impl NodeMeta {
    pub fn from_dialect(dialect: &Dialect) -> Self {
        NodeMeta {
            raw: dialect.raw as *const RawSyntaqliteDialect,
        }
    }

    fn d(&self) -> &RawSyntaqliteDialect {
        unsafe { &*self.raw }
    }

    pub fn is_list(&self, tag: u32) -> bool {
        let d = self.d();
        let idx = tag as usize;
        if idx >= d.node_count as usize {
            return false;
        }
        unsafe { *d.list_tags.add(idx) != 0 }
    }

    fn field_meta(&self, tag: u32) -> &[RawFieldMeta] {
        let d = self.d();
        let idx = tag as usize;
        if idx >= d.node_count as usize {
            return &[];
        }
        let count = unsafe { *d.field_meta_counts.add(idx) } as usize;
        let ptr = unsafe { *d.field_meta.add(idx) };
        if count == 0 || ptr.is_null() {
            return &[];
        }
        unsafe { std::slice::from_raw_parts(ptr, count) }
    }
}

/// Extract fields from a raw node pointer using C field metadata directly.
fn extract_fields<'a>(ptr: *const u8, meta: &[RawFieldMeta], source: &'a str) -> Fields<'a> {
    let mut fields = Fields::new();
    for m in meta {
        fields.push(unsafe { extract_one(ptr, m, source) });
    }
    fields
}

/// Extract a single field value from a raw node pointer.
///
/// # Safety
/// `ptr` must point to a valid node struct whose field at `m.offset` has
/// the type indicated by `m.kind`.
unsafe fn extract_one<'a>(ptr: *const u8, m: &RawFieldMeta, source: &'a str) -> FieldVal<'a> {
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

/// Format any AST node by looking up its bytecode in the dispatch table.
pub fn format_node<'a>(
    fmt: &'a LoadedFmt,
    session: &'a Session<'a>,
    node_meta: &'a NodeMeta,
    node_id: NodeId,
    arena: &mut DocArena<'a>,
) -> DocId {
    let consumed = Cell::new(0u64);
    format_node_inner(fmt, session, node_meta, node_id, arena, None, &consumed)
}

/// Format an AST node with trivia (comment) interleaving.
pub fn format_node_with_trivia<'a>(
    fmt: &'a LoadedFmt,
    session: &'a Session<'a>,
    node_meta: &'a NodeMeta,
    node_id: NodeId,
    arena: &mut DocArena<'a>,
    trivia_ctx: &'a TriviaCtx<'a>,
) -> DocId {
    let consumed = Cell::new(0u64);
    format_node_inner(
        fmt,
        session,
        node_meta,
        node_id,
        arena,
        Some(trivia_ctx),
        &consumed,
    )
}

fn format_node_inner<'a>(
    fmt: &'a LoadedFmt,
    session: &'a Session<'a>,
    node_meta: &'a NodeMeta,
    node_id: NodeId,
    arena: &mut DocArena<'a>,
    trivia_ctx: Option<&'a TriviaCtx<'a>>,
    consumed_regions: &Cell<u64>,
) -> DocId {
    if node_id.is_null() {
        return NIL_DOC;
    }

    let Some((ptr, tag)) = session.node_ptr(node_id) else {
        return NIL_DOC;
    };
    let Some(ops) = fmt.node_ops(tag) else {
        return NIL_DOC;
    };
    let ctx = fmt.ctx();
    let source = session.source();
    let macro_regions = session.macro_regions();

    let format_child = |id: NodeId, arena: &mut DocArena<'a>| {
        if !macro_regions.is_empty() {
            if let Some(verbatim) = try_macro_verbatim(
                session,
                node_meta,
                id,
                macro_regions,
                arena,
                consumed_regions,
            ) {
                return verbatim;
            }
        }
        format_node_inner(
            fmt,
            session,
            node_meta,
            id,
            arena,
            trivia_ctx,
            consumed_regions,
        )
    };
    let resolve_list = |id: NodeId| -> Vec<NodeId> {
        session
            .node_ptr(id)
            .filter(|&(_, t)| node_meta.is_list(t))
            .map(|(p, _)| {
                let list = unsafe { &*(p as *const NodeList) };
                list.children().to_vec()
            })
            .unwrap_or_default()
    };
    let children = if node_meta.is_list(tag) {
        let list = unsafe { &*(ptr as *const NodeList) };
        Some(list.children() as &[NodeId])
    } else {
        None
    };

    let meta = node_meta.field_meta(tag);
    let fields = extract_fields(ptr, meta, source);

    let trivia = trivia_ctx.map(|tc| InterpretTrivia {
        ctx: tc,
        session,
        node_meta,
    });

    interpret(
        ops,
        &ctx,
        &fields,
        children,
        arena,
        &format_child,
        &resolve_list,
        trivia.as_ref(),
    )
}

/// Compute the first (lowest) source offset of a node by scanning its fields.
/// Returns None if no non-empty source span is found.
pub fn first_source_offset(
    session: &Session<'_>,
    node_meta: &NodeMeta,
    node_id: NodeId,
) -> Option<u32> {
    if node_id.is_null() {
        return None;
    }
    let (ptr, tag) = session.node_ptr(node_id)?;

    // For list nodes, check the first child.
    if node_meta.is_list(tag) {
        let list = unsafe { &*(ptr as *const NodeList) };
        let children = list.children();
        if !children.is_empty() {
            return first_source_offset(session, node_meta, children[0]);
        }
        return None;
    }

    // Check span fields directly.
    let source = session.source();
    let meta = node_meta.field_meta(tag);
    let fields = extract_fields(ptr, meta, source);

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

    // No spans found — check child NodeId fields recursively.
    for field in fields.iter() {
        if let FieldVal::NodeId(child_id) = field {
            if let Some(off) = first_source_offset(session, node_meta, *child_id) {
                return Some(off);
            }
        }
    }

    None
}

/// Compute the last (highest) source offset end of a node by scanning its fields.
/// Returns None if no non-empty source span is found.
fn last_source_offset(
    session: &Session<'_>,
    node_meta: &NodeMeta,
    node_id: NodeId,
) -> Option<u32> {
    if node_id.is_null() {
        return None;
    }
    let (ptr, tag) = session.node_ptr(node_id)?;

    // For list nodes, check the last child.
    if node_meta.is_list(tag) {
        let list = unsafe { &*(ptr as *const NodeList) };
        let children = list.children();
        if !children.is_empty() {
            return last_source_offset(session, node_meta, children[children.len() - 1]);
        }
        return None;
    }

    // Check span fields directly — take max(offset + text.len()).
    let source = session.source();
    let meta = node_meta.field_meta(tag);
    let fields = extract_fields(ptr, meta, source);

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
                if let Some(end) = last_source_offset(session, node_meta, *child_id) {
                    max = Some(max.unwrap().max(end));
                }
            }
        }
        return max;
    }

    // No spans found — check child NodeId fields recursively.
    for field in fields.iter() {
        if let FieldVal::NodeId(child_id) = field {
            if let Some(end) = last_source_offset(session, node_meta, *child_id) {
                max = Some(match max {
                    Some(prev) => prev.max(end),
                    None => end,
                });
            }
        }
    }

    max
}

/// Check if a node's subtree overlaps with any macro region.
fn try_macro_verbatim<'a>(
    session: &'a Session<'a>,
    node_meta: &NodeMeta,
    node_id: NodeId,
    regions: &[MacroRegion],
    arena: &mut DocArena<'a>,
    consumed: &Cell<u64>,
) -> Option<DocId> {
    let first = first_source_offset(session, node_meta, node_id)?;
    let last = last_source_offset(session, node_meta, node_id)?;

    for (i, r) in regions.iter().enumerate() {
        if i >= 64 {
            break;
        }
        let r_start = r.call_offset;
        let r_end = r_start + r.call_length;

        if first < r_end && last > r_start {
            let bit = 1u64 << i;
            let bits = consumed.get();
            if bits & bit != 0 {
                return Some(NIL_DOC);
            }
            consumed.set(bits | bit);
            let source = session.source();
            let verbatim_start = first.min(r_start) as usize;
            let verbatim_end = last.max(r_end) as usize;
            return Some(arena.text(&source[verbatim_start..verbatim_end]));
        }
    }
    None
}

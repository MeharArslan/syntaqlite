use syntaqlite_parser::{NodeRef, Session, NULL_NODE};

use crate::doc::{DocArena, DocId};
use crate::interpret::{interpret, FieldVal, FmtCtx};
use crate::ops::{FieldDescriptor, FieldKind, NodeFmt};
use crate::NIL_DOC;

/// Extract typed field values from a node using field descriptors.
///
/// Reads each field at its `offset_of!`-computed byte offset and wraps
/// the raw value in the appropriate `FieldVal` variant.
fn extract_fields<'a>(
    node: NodeRef<'_>,
    source: &'a str,
    descriptors: &[FieldDescriptor],
) -> Vec<FieldVal<'a>> {
    let ptr = node.ptr() as *const u8;
    descriptors
        .iter()
        .map(|desc| {
            let p = unsafe { ptr.add(desc.offset as usize) };
            match desc.kind {
                FieldKind::NodeId => FieldVal::NodeId(unsafe { *(p as *const u32) }),
                FieldKind::Span => {
                    let o = unsafe { *(p as *const u32) } as usize;
                    let l = unsafe { *(p.add(4) as *const u16) } as usize;
                    if l == 0 {
                        FieldVal::Span("")
                    } else {
                        FieldVal::Span(&source[o..o + l])
                    }
                }
                FieldKind::Bool => FieldVal::Bool(unsafe { *(p as *const u32) } != 0),
                FieldKind::Flags => FieldVal::Flags(unsafe { *p }),
                FieldKind::Enum => FieldVal::Enum(unsafe { *(p as *const u32) }),
            }
        })
        .collect()
}

/// Format any AST node by looking up its bytecode in the dispatch table.
pub fn format_node<'a>(
    dispatch: &[Option<NodeFmt>],
    ctx: &FmtCtx<'a>,
    session: &Session<'a>,
    node_id: u32,
    arena: &mut DocArena<'a>,
) -> DocId {
    if node_id == NULL_NODE {
        return NIL_DOC;
    }
    let Some(node_ref) = session.node(node_id) else {
        return NIL_DOC;
    };
    let Some(entry) = dispatch
        .get(node_ref.tag() as usize)
        .and_then(|o| o.as_ref())
    else {
        return NIL_DOC;
    };
    let source = session.source();
    let format_child =
        |id: u32, arena: &mut DocArena<'a>| format_node(dispatch, ctx, session, id, arena);
    let resolve_list = |id: u32| -> Vec<u32> {
        session
            .node(id)
            .and_then(|n| n.as_list())
            .map(|l| l.children().to_vec())
            .unwrap_or_default()
    };
    let children = if node_ref.tag().is_list() {
        Some(node_ref.as_list().unwrap().children())
    } else {
        None
    };
    let fields = extract_fields(node_ref, source, entry.fields);
    interpret(
        entry.ops,
        ctx,
        &fields,
        children,
        arena,
        &format_child,
        &resolve_list,
    )
}

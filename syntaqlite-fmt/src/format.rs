use syntaqlite_parser::{FieldVal, Session, NULL_NODE};

use crate::doc::{DocArena, DocId};
use crate::interpret::{interpret, FmtCtx, InterpretTrivia};
use crate::ops::NodeFmt;
use crate::trivia::TriviaCtx;
use crate::NIL_DOC;

/// Format any AST node by looking up its bytecode in the dispatch table.
pub fn format_node<'a>(
    dispatch: &'a [Option<NodeFmt>],
    ctx: &FmtCtx<'a>,
    session: &'a Session<'a>,
    node_id: u32,
    arena: &mut DocArena<'a>,
) -> DocId {
    format_node_inner(dispatch, ctx, session, node_id, arena, None)
}

/// Format an AST node with trivia (comment) interleaving.
pub fn format_node_with_trivia<'a>(
    dispatch: &'a [Option<NodeFmt>],
    ctx: &FmtCtx<'a>,
    session: &'a Session<'a>,
    node_id: u32,
    arena: &mut DocArena<'a>,
    trivia_ctx: &'a TriviaCtx<'a>,
) -> DocId {
    format_node_inner(dispatch, ctx, session, node_id, arena, Some(trivia_ctx))
}

fn format_node_inner<'a>(
    dispatch: &'a [Option<NodeFmt>],
    ctx: &FmtCtx<'a>,
    session: &'a Session<'a>,
    node_id: u32,
    arena: &mut DocArena<'a>,
    trivia_ctx: Option<&'a TriviaCtx<'a>>,
) -> DocId {
    if node_id == NULL_NODE {
        return NIL_DOC;
    }
    let Some(node) = session.node(node_id) else {
        return NIL_DOC;
    };
    let Some(entry) = dispatch
        .get(node.tag() as usize)
        .and_then(|o| o.as_ref())
    else {
        return NIL_DOC;
    };
    let source = session.source();
    let format_child = |id: u32, arena: &mut DocArena<'a>| {
        format_node_inner(dispatch, ctx, session, id, arena, trivia_ctx)
    };
    let resolve_list = |id: u32| -> Vec<u32> {
        session
            .node(id)
            .and_then(|n| n.as_list())
            .map(|l| l.children().to_vec())
            .unwrap_or_default()
    };
    let children = if node.tag().is_list() {
        Some(node.as_list().unwrap().children())
    } else {
        None
    };

    let fields = node.fields(source);

    let trivia = trivia_ctx.map(|tc| InterpretTrivia {
        ctx: tc,
        dispatch,
        session,
    });

    interpret(
        entry.ops,
        ctx,
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
    dispatch: &[Option<NodeFmt>],
    session: &Session<'_>,
    node_id: u32,
) -> Option<u32> {
    if node_id == NULL_NODE {
        return None;
    }
    let node = session.node(node_id)?;

    // For list nodes, check the first child.
    if node.tag().is_list() {
        let list = node.as_list()?;
        let children = list.children();
        if !children.is_empty() {
            return first_source_offset(dispatch, session, children[0]);
        }
        return None;
    }

    // Check span fields directly.
    let source = session.source();
    let fields = node.fields(source);

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
            if let Some(off) = first_source_offset(dispatch, session, *child_id) {
                return Some(off);
            }
        }
    }

    None
}

use std::cell::Cell;

use syntaqlite_parser::{FieldVal, MacroRegion, Session, NULL_NODE};

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

    // Check if this node falls entirely within a macro region.
    // If so, emit the original macro call text verbatim.
    let macro_regions = session.macro_regions();
    if !macro_regions.is_empty() {
        if let Some(verbatim) =
            try_macro_verbatim(dispatch, session, node_id, macro_regions, arena)
        {
            return verbatim;
        }
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

/// Compute the last (highest) source offset end of a node by scanning its fields.
/// Returns None if no non-empty source span is found.
pub fn last_source_offset(
    dispatch: &[Option<NodeFmt>],
    session: &Session<'_>,
    node_id: u32,
) -> Option<u32> {
    if node_id == NULL_NODE {
        return None;
    }
    let node = session.node(node_id)?;

    // For list nodes, check the last child.
    if node.tag().is_list() {
        let list = node.as_list()?;
        let children = list.children();
        if !children.is_empty() {
            return last_source_offset(dispatch, session, children[children.len() - 1]);
        }
        return None;
    }

    // Check span fields directly — take max(offset + text.len()).
    let source = session.source();
    let fields = node.fields(source);

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
        // Also check child nodes — they may extend beyond direct spans.
        for field in fields.iter() {
            if let FieldVal::NodeId(child_id) = field {
                if let Some(end) = last_source_offset(dispatch, session, *child_id) {
                    max = Some(max.unwrap().max(end));
                }
            }
        }
        return max;
    }

    // No spans found — check child NodeId fields recursively.
    for field in fields.iter() {
        if let FieldVal::NodeId(child_id) = field {
            if let Some(end) = last_source_offset(dispatch, session, *child_id) {
                max = Some(match max {
                    Some(prev) => prev.max(end),
                    None => end,
                });
            }
        }
    }

    max
}

/// If the node's source span falls entirely within a macro region, return a
/// verbatim text doc of the original macro call. Otherwise return None.
fn try_macro_verbatim<'a>(
    dispatch: &[Option<NodeFmt>],
    session: &'a Session<'a>,
    node_id: u32,
    regions: &[MacroRegion],
    arena: &mut DocArena<'a>,
) -> Option<DocId> {
    let first = first_source_offset(dispatch, session, node_id)?;
    let last = last_source_offset(dispatch, session, node_id)?;
    let source = session.source();
    for r in regions {
        let r_end = r.call_offset + r.call_length;
        if first >= r.call_offset && last <= r_end {
            return Some(arena.text(&source[r.call_offset as usize..r_end as usize]));
        }
    }
    None
}

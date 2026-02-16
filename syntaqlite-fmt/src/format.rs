use std::cell::Cell;

use syntaqlite_parser::{MacroRegion, NodeId, Session};

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
    node_id: NodeId,
    arena: &mut DocArena<'a>,
) -> DocId {
    let consumed = Cell::new(0u64);
    format_node_inner(dispatch, ctx, session, node_id, arena, None, &consumed)
}

/// Format an AST node with trivia (comment) interleaving.
pub fn format_node_with_trivia<'a>(
    dispatch: &'a [Option<NodeFmt>],
    ctx: &FmtCtx<'a>,
    session: &'a Session<'a>,
    node_id: NodeId,
    arena: &mut DocArena<'a>,
    trivia_ctx: &'a TriviaCtx<'a>,
) -> DocId {
    let consumed = Cell::new(0u64);
    format_node_inner(
        dispatch,
        ctx,
        session,
        node_id,
        arena,
        Some(trivia_ctx),
        &consumed,
    )
}

fn format_node_inner<'a>(
    dispatch: &'a [Option<NodeFmt>],
    ctx: &FmtCtx<'a>,
    session: &'a Session<'a>,
    node_id: NodeId,
    arena: &mut DocArena<'a>,
    trivia_ctx: Option<&'a TriviaCtx<'a>>,
    consumed_regions: &Cell<u64>,
) -> DocId {
    if node_id.is_null() {
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
    let macro_regions = session.macro_regions();

    // The format_child closure checks for macro overlap. If a child's subtree
    // overlaps with any macro region, emit the source text verbatim instead
    // of recursing into the child's formatting. This keeps the macro check at
    // the child level so that parent keywords (SELECT, FROM, etc.) are still
    // emitted by bytecode.
    let format_child = |id: NodeId, arena: &mut DocArena<'a>| {
        if !macro_regions.is_empty() {
            if let Some(verbatim) = try_macro_verbatim(
                dispatch,
                session,
                id,
                macro_regions,
                arena,
                consumed_regions,
            ) {
                return verbatim;
            }
        }
        format_node_inner(dispatch, ctx, session, id, arena, trivia_ctx, consumed_regions)
    };
    let resolve_list = |id: NodeId| -> Vec<NodeId> {
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
    node_id: NodeId,
) -> Option<u32> {
    use syntaqlite_parser::FieldVal;

    if node_id.is_null() {
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
    node_id: NodeId,
) -> Option<u32> {
    use syntaqlite_parser::FieldVal;

    if node_id.is_null() {
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

/// Check if a node's subtree overlaps with any macro region. If so:
/// - First overlap for a region: emit verbatim text covering the union of the
///   node's source range and the overlapping macro call range(s), mark consumed
/// - Subsequent overlaps for the same region: return NIL_DOC (suppress)
/// - No overlap: return None (format normally)
fn try_macro_verbatim<'a>(
    dispatch: &[Option<NodeFmt>],
    session: &'a Session<'a>,
    node_id: NodeId,
    regions: &[MacroRegion],
    arena: &mut DocArena<'a>,
    consumed: &Cell<u64>,
) -> Option<DocId> {
    let first = first_source_offset(dispatch, session, node_id)?;
    let last = last_source_offset(dispatch, session, node_id)?;

    for (i, r) in regions.iter().enumerate() {
        if i >= 64 {
            break; // Bitset limit
        }
        let r_start = r.call_offset;
        let r_end = r_start + r.call_length;

        // Check overlap: node's span range [first, last] intersects
        // macro region [r_start, r_end].
        if first < r_end && last > r_start {
            let bit = 1u64 << i;
            let bits = consumed.get();
            if bits & bit != 0 {
                // Already emitted — suppress this node.
                return Some(NIL_DOC);
            }
            // First overlap — emit verbatim. The range is the union of the
            // node's source range and the macro call range.
            consumed.set(bits | bit);
            let source = session.source();
            let verbatim_start = first.min(r_start) as usize;
            let verbatim_end = last.max(r_end) as usize;
            return Some(arena.text(&source[verbatim_start..verbatim_end]));
        }
    }
    None
}

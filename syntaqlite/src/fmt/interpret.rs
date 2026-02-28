// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use crate::dialect::Dialect;
use crate::parser::{CursorBase, FieldVal, Fields, NodeId};

use super::bytecode::opcodes;
use super::comment::{CommentCtx, DrainResult};
use super::doc::{DocArena, DocId, NIL_DOC};

/// Shared context threaded through the recursive formatting tree.
/// Bundles the state that is constant across all nodes in a single format call.
pub(crate) struct FmtCtx<'a> {
    pub dialect: Dialect<'a>,
    pub cursor: &'a CursorBase<'a>,
    pub comment_ctx: Option<&'a CommentCtx<'a>>,
}

impl<'a> FmtCtx<'a> {
    pub fn source(&self) -> &'a str {
        self.cursor.source()
    }
}

/// Reusable scratch buffers for the iterative interpret loop, allocated
/// once per `Formatter`. The `gn_stack` is shared across all nodes within
/// a single format call.
pub(crate) struct InterpretScratch {
    gn_stack: Vec<GNFrame>,
}

impl InterpretScratch {
    pub fn new() -> Self {
        InterpretScratch {
            gn_stack: Vec::new(),
        }
    }
}

// Old recursive `interpret` and `format_child_doc` removed —
// ── Iterative interpreter ────────────────────────────────────────────────

/// Saved execution state of a parent node when "calling" into a child.
///
/// Stores `node_id` instead of the full `Fields<'a>` array (392 bytes)
/// to keep the frame small (~80 bytes). Fields are cheaply re-extracted
/// via `cursor.node_ptr(node_id)` when the parent frame is restored.
struct CallFrame<'a> {
    ops: &'a [u8],
    ops_count: usize,
    ip: usize,
    /// Parent node ID — used to re-derive (ptr, tag) and re-extract fields on pop.
    node_id: NodeId,
    list_children: Option<&'a [NodeId]>,
    running: DocId,
    pending: DocId,
    gn_save: usize,
    fe_save: usize,
    return_action: ReturnAction,
}

/// What to do with the child's result when returning to the parent.
enum ReturnAction {
    /// Cat child result onto parent's running accumulator.
    CatOntoRunning,
    /// Discard child result (macro verbatim/suppressed already handled).
    Discard,
}

/// Iterative entry point — replaces the recursive `format_node_inner` →
/// `interpret` → `format_child_doc` → `format_node_inner` chain.
///
/// Uses an explicit call stack (`scratch.call_stack`) instead of native
/// recursion. Each `Child(idx)` or `ChildItem` op pushes a `CallFrame`
/// and sets up the child's execution state; when a node finishes
/// (`ip >= ops_count`), the parent's state is restored from the stack.
pub(crate) fn interpret_node<'a>(
    ctx: &FmtCtx<'a>,
    root_id: NodeId,
    consumed_regions: &mut u64,
    arena: &mut DocArena<'a>,
    scratch: &mut InterpretScratch,
) -> DocId {
    if root_id.is_null() {
        return NIL_DOC;
    }

    // Look up root node.
    let Some((ptr, tag)) = ctx.cursor.node_ptr(root_id) else {
        return NIL_DOC;
    };
    let Some((ops_bytes, ops_len)) = ctx.dialect.fmt_dispatch(tag) else {
        return NIL_DOC;
    };
    let children = ctx.cursor.list_children(root_id, &ctx.dialect);
    let source = ctx.source();
    let fields = super::formatter::extract_fields(&ctx.dialect, ptr, tag, source);

    // Local stacks with correct lifetime 'a — no transmute needed.
    let mut call_stack: Vec<CallFrame<'a>> = Vec::new();
    let mut for_each_stack: Vec<ForEachState<'a>> = Vec::new();

    // Current node's execution state.
    let mut cur_node_id: NodeId = root_id;
    let mut ops: &[u8] = &ops_bytes[..ops_len * 6];
    let mut ops_count: usize = ops_len;
    let mut fields: Fields<'a> = fields;
    let mut list_children: Option<&[NodeId]> = children;
    let mut running: DocId = NIL_DOC;
    let mut pending: DocId = NIL_DOC;
    let mut gn_save = scratch.gn_stack.len();
    let mut fe_save = for_each_stack.len();
    let mut ip: usize = 0;
    let has_comments = ctx.comment_ctx.is_some();

    /// Push a call frame onto the call stack, setting up child execution state.
    /// Uses `continue` to restart the interpreter loop for the child node.
    macro_rules! push_call_frame {
        ($child_id:expr, $child_ops_bytes:expr, $child_ops_len:expr,
         $child_fields:expr, $child_children:expr, $return_action_val:expr) => {{
            let frame = CallFrame {
                ops,
                ops_count,
                ip: ip + 1,
                node_id: cur_node_id,
                list_children,
                running,
                pending,
                gn_save,
                fe_save,
                return_action: $return_action_val,
            };
            call_stack.push(frame);

            cur_node_id = $child_id;
            ops = &$child_ops_bytes[..$child_ops_len * 6];
            ops_count = $child_ops_len;
            fields = $child_fields;
            list_children = $child_children;
            running = NIL_DOC;
            pending = NIL_DOC;
            gn_save = scratch.gn_stack.len();
            fe_save = for_each_stack.len();
            ip = 0;
            continue;
        }};
    }

    loop {
        if ip >= ops_count {
            // Current node is done. Finalize its doc.
            let result = arena.cat(running, pending);
            scratch.gn_stack.truncate(gn_save);
            for_each_stack.truncate(fe_save);

            if call_stack.is_empty() {
                // Back to root — return final result.
                return result;
            }

            // Pop parent frame.
            let frame = call_stack.pop().unwrap();
            cur_node_id = frame.node_id;
            ops = frame.ops;
            ops_count = frame.ops_count;
            ip = frame.ip;
            // Re-extract fields from the saved node_id — cheap lookups.
            let (rptr, rtag) = ctx.cursor.node_ptr(cur_node_id).unwrap();
            fields = super::formatter::extract_fields(&ctx.dialect, rptr, rtag, source);
            list_children = frame.list_children;
            running = frame.running;
            pending = frame.pending;
            gn_save = frame.gn_save;
            fe_save = frame.fe_save;

            match frame.return_action {
                ReturnAction::CatOntoRunning => {
                    running = arena.cat(running, result);
                }
                ReturnAction::Discard => {}
            }
            continue;
        }

        match op_at(ops, ip) {
            FmtOp::Keyword(sid) => {
                let kw_text = ctx.dialect.fmt_string(sid);

                if let Some(cctx) = ctx.comment_ctx {
                    if let Some((tok_offset, word_count)) = cctx.peek_keyword_tokens(kw_text) {
                        let drain = cctx.drain_before(tok_offset, source, arena);
                        flush_drain(drain, &mut pending, &mut running, arena);
                        cctx.advance_token_cursor(word_count);
                    } else {
                        running = arena.cat(running, pending);
                        pending = NIL_DOC;
                    }
                }
                let kw = arena.keyword(kw_text);
                running = arena.cat(running, kw);
            }
            FmtOp::Span(idx) => {
                let FieldVal::Span(s, offset) = fields[idx as usize] else {
                    panic!("Span: field {} is not a Span", idx);
                };

                if !s.is_empty() {
                    if let Some(cctx) = ctx.comment_ctx {
                        let drain = cctx.drain_before(offset, source, arena);
                        flush_drain(drain, &mut pending, &mut running, arena);
                        cctx.advance_past(offset + s.len() as u32);
                    }
                    let txt = arena.text(s);
                    running = arena.cat(running, txt);
                }
            }
            FmtOp::Child(idx) => {
                let FieldVal::NodeId(child_id) = fields[idx as usize] else {
                    panic!("Child: field {} is not a NodeId", idx);
                };

                if !child_id.is_null() {
                    // Drain comments before this child.
                    if let Some(cctx) = ctx.comment_ctx {
                        if let Some((offset, _)) = cctx.peek_next_token() {
                            let drain = cctx.drain_before(offset, source, arena);
                            flush_drain(drain, &mut pending, &mut running, arena);
                        } else {
                            running = arena.cat(running, pending);
                            pending = NIL_DOC;
                        }
                    }

                    // Check macro verbatim for non-list children.
                    let mut return_action = ReturnAction::CatOntoRunning;
                    let macro_regions = ctx.cursor.macro_regions();
                    if !macro_regions.is_empty()
                        && ctx.cursor.list_children(child_id, &ctx.dialect).is_none()
                        && let Some(doc) = super::formatter::try_macro_verbatim(
                            ctx,
                            macro_regions,
                            arena,
                            consumed_regions,
                        )
                    {
                        // Verbatim doc already added to running; still need to
                        // "call" the child to advance comment cursor, but discard
                        // its result.
                        running = arena.cat(running, doc);
                        return_action = ReturnAction::Discard;
                    }

                    // "Call" child: push parent frame, set up child state.
                    if let Some((cptr, ctag)) = ctx.cursor.node_ptr(child_id)
                        && let Some((child_ops_bytes, child_ops_len)) =
                            ctx.dialect.fmt_dispatch(ctag)
                    {
                        let child_children = ctx.cursor.list_children(child_id, &ctx.dialect);
                        let child_fields =
                            super::formatter::extract_fields(&ctx.dialect, cptr, ctag, source);
                        push_call_frame!(
                            child_id,
                            child_ops_bytes,
                            child_ops_len,
                            child_fields,
                            child_children,
                            return_action
                        );
                    }
                }
            }
            FmtOp::Line => {
                let l = arena.line();
                if has_comments {
                    pending = arena.cat(pending, l);
                } else {
                    running = arena.cat(running, l);
                }
            }
            FmtOp::SoftLine => {
                let sl = arena.softline();
                if has_comments {
                    pending = arena.cat(pending, sl);
                } else {
                    running = arena.cat(running, sl);
                }
            }
            FmtOp::HardLine => {
                let hl = arena.hardline();
                if has_comments {
                    pending = arena.cat(pending, hl);
                } else {
                    running = arena.cat(running, hl);
                }
            }
            FmtOp::GroupStart => {
                scratch.gn_stack.push(GNFrame::Group(running));
                running = NIL_DOC;
            }
            FmtOp::GroupEnd => {
                running = arena.cat(running, pending);
                pending = NIL_DOC;
                let inner = running;
                match scratch.gn_stack.pop().expect("unmatched GroupEnd") {
                    GNFrame::Group(parent) => {
                        let g = arena.group(inner);
                        running = arena.cat(parent, g);
                    }
                    _ => panic!("expected Group frame"),
                }
            }
            FmtOp::NestStart(indent) => {
                scratch.gn_stack.push(GNFrame::Nest(indent, running));
                running = NIL_DOC;
            }
            FmtOp::NestEnd => {
                running = arena.cat(running, pending);
                pending = NIL_DOC;
                let inner = running;
                match scratch.gn_stack.pop().expect("unmatched NestEnd") {
                    GNFrame::Nest(indent, parent) => {
                        let n = arena.nest(indent, inner);
                        running = arena.cat(parent, n);
                    }
                    _ => panic!("expected Nest frame"),
                }
            }
            FmtOp::IfSet(idx, skip) => {
                let FieldVal::NodeId(id) = fields[idx as usize] else {
                    panic!("IfSet: field {} is not a NodeId", idx);
                };
                if id.is_null() {
                    ip += skip as usize;
                }
            }
            FmtOp::Else(skip) => {
                ip += skip as usize;
            }
            FmtOp::EndIf => {}
            FmtOp::ForEachStart(idx) => {
                let FieldVal::NodeId(list_id) = fields[idx as usize] else {
                    panic!("ForEachStart: field {} is not a NodeId", idx);
                };
                if list_id.is_null() {
                    ip = skip_to_foreach_end(ops, ops_count, ip);
                } else {
                    let children = ctx
                        .cursor
                        .list_children(list_id, &ctx.dialect)
                        .unwrap_or(&[]);
                    if children.is_empty() {
                        ip = skip_to_foreach_end(ops, ops_count, ip);
                    } else {
                        let state = ForEachState {
                            children,
                            index: 0,
                            body_start: ip + 1,
                            sep_checkpoint: None,
                        };
                        for_each_stack.push(state);
                    }
                }
            }
            FmtOp::ChildItem => {
                let state = for_each_stack.last().expect("ChildItem outside ForEach");
                let child_id = state.children[state.index];

                // Check macro suppression BEFORE draining comments.
                let macro_regions = ctx.cursor.macro_regions();
                let macro_doc = if !macro_regions.is_empty()
                    && ctx.cursor.list_children(child_id, &ctx.dialect).is_none()
                {
                    super::formatter::try_macro_verbatim(
                        ctx,
                        macro_regions,
                        arena,
                        consumed_regions,
                    )
                } else {
                    None
                };

                let return_action;

                if macro_doc == Some(NIL_DOC) {
                    // Macro-suppressed child. Undo the previous separator
                    // and discard pending line breaks.
                    let state = for_each_stack.last_mut().unwrap();
                    if let Some((saved_running, saved_pending)) = state.sep_checkpoint.take() {
                        running = saved_running;
                        pending = saved_pending;
                    }
                    return_action = ReturnAction::Discard;
                } else {
                    // Drain comments before this child.
                    if let Some(cctx) = ctx.comment_ctx {
                        if let Some((offset, _)) = cctx.peek_next_token() {
                            let drain = cctx.drain_before(offset, source, arena);
                            flush_drain(drain, &mut pending, &mut running, arena);
                        } else {
                            running = arena.cat(running, pending);
                            pending = NIL_DOC;
                        }
                    }

                    if let Some(verbatim) = macro_doc {
                        // Macro-verbatim: add verbatim text, still "call" child to
                        // advance comment cursor, but discard child result.
                        running = arena.cat(running, verbatim);
                        return_action = ReturnAction::Discard;
                    } else {
                        return_action = ReturnAction::CatOntoRunning;
                    }
                }

                // "Call" child: push parent frame, set up child state.
                if let Some((cptr, ctag)) = ctx.cursor.node_ptr(child_id)
                    && let Some((child_ops_bytes, child_ops_len)) = ctx.dialect.fmt_dispatch(ctag)
                {
                    let child_children = ctx.cursor.list_children(child_id, &ctx.dialect);
                    let child_fields =
                        super::formatter::extract_fields(&ctx.dialect, cptr, ctag, source);
                    push_call_frame!(
                        child_id,
                        child_ops_bytes,
                        child_ops_len,
                        child_fields,
                        child_children,
                        return_action
                    );
                }
            }
            FmtOp::ForEachSep(sid) => {
                let state = for_each_stack
                    .last_mut()
                    .expect("ForEachSep outside ForEach");
                if state.index < state.children.len() - 1 {
                    state.sep_checkpoint = Some((running, pending));
                    let sep_text = ctx.dialect.fmt_string(sid);
                    if let Some(cctx) = ctx.comment_ctx
                        && let Some((_, word_count)) = cctx.peek_keyword_tokens(sep_text)
                    {
                        cctx.advance_token_cursor(word_count);
                    }
                    let sep = arena.text(sep_text);
                    running = arena.cat(running, sep);
                } else {
                    ip = skip_to_foreach_end(ops, ops_count, ip);
                    continue;
                }
            }
            FmtOp::ForEachEnd => {
                let state = for_each_stack
                    .last_mut()
                    .expect("ForEachEnd outside ForEach");
                state.index += 1;
                if state.index < state.children.len() {
                    ip = state.body_start;
                    continue;
                } else {
                    for_each_stack.pop();
                }
            }
            FmtOp::IfBool(idx, skip) => {
                let FieldVal::Bool(val) = fields[idx as usize] else {
                    panic!("IfBool: field {} is not a Bool", idx);
                };
                if !val {
                    ip += skip as usize;
                }
            }
            FmtOp::IfFlag(idx, mask, skip) => {
                let FieldVal::Flags(f) = fields[idx as usize] else {
                    panic!("IfFlag: field {} is not Flags", idx);
                };
                if f & mask == 0 {
                    ip += skip as usize;
                }
            }
            FmtOp::IfEnum(idx, ordinal, skip) => {
                let FieldVal::Enum(val) = fields[idx as usize] else {
                    panic!("IfEnum: field {} is not an Enum", idx);
                };
                if val != ordinal as u32 {
                    ip += skip as usize;
                }
            }
            FmtOp::IfSpan(idx, skip) => {
                let FieldVal::Span(s, _) = fields[idx as usize] else {
                    panic!("IfSpan: field {} is not a Span", idx);
                };
                if s.is_empty() {
                    ip += skip as usize;
                }
            }
            FmtOp::EnumDisplay(idx, base) => {
                let FieldVal::Enum(ordinal) = fields[idx as usize] else {
                    panic!("EnumDisplay: field {} is not an Enum", idx);
                };
                let string_id = ctx
                    .dialect
                    .fmt_enum_display_val(base as usize + ordinal as usize);
                let kw_text = ctx.dialect.fmt_string(string_id);
                if let Some(cctx) = ctx.comment_ctx {
                    if let Some((tok_offset, word_count)) = cctx.peek_keyword_tokens(kw_text) {
                        let drain = cctx.drain_before(tok_offset, source, arena);
                        flush_drain(drain, &mut pending, &mut running, arena);
                        cctx.advance_token_cursor(word_count);
                    } else {
                        running = arena.cat(running, pending);
                        pending = NIL_DOC;
                    }
                }
                let kw = arena.keyword(kw_text);
                running = arena.cat(running, kw);
            }
            FmtOp::ForEachSelfStart => {
                let children = list_children.expect("ForEachSelfStart on non-list node");
                if children.is_empty() {
                    ip = skip_to_foreach_end(ops, ops_count, ip);
                } else {
                    let state = ForEachState {
                        children,
                        index: 0,
                        body_start: ip + 1,
                        sep_checkpoint: None,
                    };
                    for_each_stack.push(state);
                }
            }
        }
        ip += 1;
    }
}

// ── Comment drain helper ────────────────────────────────────────────────

/// Flush a `DrainResult` into the running/pending accumulators.
#[inline]
fn flush_drain(drain: DrainResult, pending: &mut DocId, running: &mut DocId, arena: &mut DocArena) {
    if drain.trailing != NIL_DOC {
        *running = arena.cat(*running, drain.trailing);
    }
    if drain.leading != NIL_DOC {
        *pending = NIL_DOC;
        *running = arena.cat(*running, drain.leading);
    } else {
        *running = arena.cat(*running, *pending);
        *pending = NIL_DOC;
    }
}

// ── Bytecode helpers ────────────────────────────────────────────────────

/// Decode the op at position `ip` from a raw byte stream.
#[inline(always)]
fn op_at(ops: &[u8], ip: usize) -> FmtOp {
    let base = ip * 6;
    let opcode = ops[base];
    let a = ops[base + 1];
    let b = u16::from_le_bytes([ops[base + 2], ops[base + 3]]);
    let c = u16::from_le_bytes([ops[base + 4], ops[base + 5]]);
    FmtOp::decode(opcode, a, b, c)
}

/// Find the matching ForEachEnd for the ForEach at `from_ip` via linear scan.
/// ForEach bodies are typically very short (5-15 ops), so a linear scan is
/// faster than building and indexing a per-node jump table (which requires
/// a `vec![0; ops_count]` allocation).
fn skip_to_foreach_end(ops: &[u8], ops_count: usize, from_ip: usize) -> usize {
    let mut depth = 1u32;
    let mut ip = from_ip + 1;
    while ip < ops_count {
        match op_at(ops, ip) {
            FmtOp::ForEachStart(_) | FmtOp::ForEachSelfStart => depth += 1,
            FmtOp::ForEachEnd => {
                depth -= 1;
                if depth == 0 {
                    return ip;
                }
            }
            _ => {}
        }
        ip += 1;
    }
    panic!("unmatched ForEachStart");
}

// ── Private types ───────────────────────────────────────────────────────

type StringId = u16;
type FieldIdx = u16;
type SkipCount = u16;

/// Typed representation of a single formatting opcode.
/// Decoded from raw 6-byte ops via `FmtOp::decode()` (a `const fn`,
/// so LLVM resolves the decode match at compile time when the opcode
/// is statically known in each match arm).
#[derive(Debug, Clone, Copy, PartialEq)]
enum FmtOp {
    /// Emit a keyword from the string table.
    Keyword(StringId),
    /// Emit source text from a Span field.
    Span(FieldIdx),
    /// Recursively format the child node whose ID is in a NodeId field.
    /// Skipped if the child ID is NULL_NODE.
    Child(FieldIdx),
    /// Flat: space. Break: newline + indent.
    Line,
    /// Flat: empty. Break: newline + indent.
    SoftLine,
    /// Always newline + indent.
    HardLine,
    /// Begin a group (try flat, break if doesn't fit).
    GroupStart,
    /// End a group.
    GroupEnd,
    /// Begin indentation nest.
    NestStart(i16),
    /// End indentation nest.
    NestEnd,
    /// If NodeId field != NULL_NODE, execute next ops; else skip.
    IfSet(FieldIdx, SkipCount),
    /// End of then-branch. If reached, skip the else-branch.
    Else(SkipCount),
    /// No-op marker ending a conditional block.
    EndIf,
    /// Begin iterating children of the list node referenced by a NodeId field.
    ForEachStart(FieldIdx),
    /// Format the current iteration child.
    ChildItem,
    /// Emit separator text between list items (not after last).
    ForEachSep(StringId),
    /// End of ForEach body.
    ForEachEnd,
    /// If Bool field is true, execute next ops; else skip.
    IfBool(FieldIdx, SkipCount),
    /// If Flags field has (value & mask) != 0, execute next ops; else skip.
    IfFlag(FieldIdx, u8, SkipCount),
    /// If Enum field == variant ordinal, execute next ops; else skip.
    IfEnum(FieldIdx, u16, SkipCount),
    /// If Span field is non-empty, execute next ops; else skip.
    IfSpan(FieldIdx, SkipCount),
    /// Map enum ordinal → string via lookup table. `u16` is base index into enum_display table.
    EnumDisplay(FieldIdx, u16),
    /// Begin iterating children of self (for list nodes).
    ForEachSelfStart,
}

impl FmtOp {
    /// Decode a raw opcode tuple into a typed `FmtOp`.
    #[inline(always)]
    pub const fn decode(opcode: u8, a: u8, b: u16, c: u16) -> Self {
        match opcode {
            opcodes::KEYWORD => FmtOp::Keyword(b),
            opcodes::SPAN => FmtOp::Span(a as u16),
            opcodes::CHILD => FmtOp::Child(a as u16),
            opcodes::LINE => FmtOp::Line,
            opcodes::SOFTLINE => FmtOp::SoftLine,
            opcodes::HARDLINE => FmtOp::HardLine,
            opcodes::GROUP_START => FmtOp::GroupStart,
            opcodes::GROUP_END => FmtOp::GroupEnd,
            opcodes::NEST_START => FmtOp::NestStart(b as i16),
            opcodes::NEST_END => FmtOp::NestEnd,
            opcodes::IF_SET => FmtOp::IfSet(a as u16, c),
            opcodes::ELSE_OP => FmtOp::Else(c),
            opcodes::END_IF => FmtOp::EndIf,
            opcodes::FOR_EACH_START => FmtOp::ForEachStart(a as u16),
            opcodes::CHILD_ITEM => FmtOp::ChildItem,
            opcodes::FOR_EACH_SEP => FmtOp::ForEachSep(b),
            opcodes::FOR_EACH_END => FmtOp::ForEachEnd,
            opcodes::IF_BOOL => FmtOp::IfBool(a as u16, c),
            opcodes::IF_FLAG => FmtOp::IfFlag(a as u16, b as u8, c),
            opcodes::IF_ENUM => FmtOp::IfEnum(a as u16, b, c),
            opcodes::IF_SPAN => FmtOp::IfSpan(a as u16, c),
            opcodes::ENUM_DISPLAY => FmtOp::EnumDisplay(a as u16, b),
            opcodes::FOR_EACH_SELF_START => FmtOp::ForEachSelfStart,
            _ => panic!("unknown opcode in fmt data"),
        }
    }
}

/// Group/Nest frame — saves the parent's running DocId.
enum GNFrame {
    Group(DocId),
    Nest(i16, DocId),
}

struct ForEachState<'a> {
    children: &'a [NodeId],
    index: usize,
    body_start: usize,
    /// Saved `(running, pending)` before the separator was emitted.
    /// If the next ChildItem is macro-suppressed, restore these to undo
    /// the separator (the orphaned Doc nodes remain in the arena but are
    /// unreachable and harmless).
    sep_checkpoint: Option<(DocId, DocId)>,
}

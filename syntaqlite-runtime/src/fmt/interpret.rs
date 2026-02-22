// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use crate::dialect::Dialect;
use crate::parser::{CursorBase, FieldVal, NodeId};

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

/// Interpret formatting bytecode into a Doc tree.
///
/// Instead of collecting DocIds into a `Vec<DocId>` and calling `cats()` at
/// the end, this builds a left-leaning Cat chain directly in the arena via a
/// single `running: DocId` accumulator.  GroupStart/GroupEnd and
/// NestStart/NestEnd save/restore the accumulator on a tiny stack of DocIds
/// (no Vec-of-DocIds).  This eliminates the dominant source of `grow_one`
/// allocator pressure in the formatter hot path.
pub(crate) fn interpret<'a>(
    ctx: &FmtCtx<'a>,
    ops_bytes: &[u8],
    ops_count: usize,
    fields: &[FieldVal<'a>],
    list_children: Option<&[NodeId]>,
    consumed_regions: &mut u64,
    arena: &mut DocArena<'a>,
) -> DocId {
    let ops = &ops_bytes[..ops_count * 6];
    let source = ctx.source();

    // Running accumulator — replaces `parts: Vec<DocId>`.
    let mut running: DocId = NIL_DOC;
    // Pending line breaks — replaces `pending_lines: Vec<DocId>`.
    let mut pending: DocId = NIL_DOC;

    let mut gn_stack: Vec<GNFrame> = Vec::new();
    let mut for_each_stack: Vec<ForEachState> = Vec::new();
    let mut ip: usize = 0;
    let has_comments = ctx.comment_ctx.is_some();
    let jump_table = build_foreach_jump_table(ops, ops_count);

    while ip < ops_count {
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
                    if let Some(cctx) = ctx.comment_ctx {
                        if let Some((offset, _)) = cctx.peek_next_token() {
                            let drain = cctx.drain_before(offset, source, arena);
                            flush_drain(drain, &mut pending, &mut running, arena);
                        } else {
                            running = arena.cat(running, pending);
                            pending = NIL_DOC;
                        }
                    }
                    let child_doc = format_child_doc(ctx, child_id, consumed_regions, arena);
                    running = arena.cat(running, child_doc);
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
                gn_stack.push(GNFrame::Group(running));
                running = NIL_DOC;
            }
            FmtOp::GroupEnd => {
                running = arena.cat(running, pending);
                pending = NIL_DOC;
                let inner = running;
                match gn_stack.pop().expect("unmatched GroupEnd") {
                    GNFrame::Group(parent) => {
                        let g = arena.group(inner);
                        running = arena.cat(parent, g);
                    }
                    _ => panic!("expected Group frame"),
                }
            }
            FmtOp::NestStart(indent) => {
                gn_stack.push(GNFrame::Nest(indent, running));
                running = NIL_DOC;
            }
            FmtOp::NestEnd => {
                running = arena.cat(running, pending);
                pending = NIL_DOC;
                let inner = running;
                match gn_stack.pop().expect("unmatched NestEnd") {
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
                    ip = skip_to_foreach_end(ops, ops_count, ip, &jump_table);
                } else {
                    let children = ctx
                        .cursor
                        .list_children(list_id, &ctx.dialect)
                        .unwrap_or(&[]);
                    if children.is_empty() {
                        ip = skip_to_foreach_end(ops, ops_count, ip, &jump_table);
                    } else {
                        for_each_stack.push(ForEachState {
                            children,
                            index: 0,
                            body_start: ip + 1,
                            sep_checkpoint: None,
                        });
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

                if macro_doc == Some(NIL_DOC) {
                    // Macro-suppressed child. Undo the previous separator
                    // and discard pending line breaks.
                    let state = for_each_stack.last_mut().unwrap();
                    if let Some((saved_running, saved_pending)) = state.sep_checkpoint.take() {
                        running = saved_running;
                        pending = saved_pending;
                    }
                    let _ =
                        super::formatter::format_node_inner(ctx, child_id, arena, consumed_regions);
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
                        let _ = super::formatter::format_node_inner(
                            ctx,
                            child_id,
                            arena,
                            consumed_regions,
                        );
                        running = arena.cat(running, verbatim);
                    } else {
                        let child_doc = format_child_doc(ctx, child_id, consumed_regions, arena);
                        running = arena.cat(running, child_doc);
                    }
                }
            }
            FmtOp::ForEachSep(sid) => {
                let state = for_each_stack
                    .last_mut()
                    .expect("ForEachSep outside ForEach");
                if state.index < state.children.len() - 1 {
                    state.sep_checkpoint = Some((running, pending));
                    let sep_text = ctx.dialect.fmt_string(sid);
                    if let Some(cctx) = ctx.comment_ctx {
                        if let Some((_, word_count)) = cctx.peek_keyword_tokens(sep_text) {
                            cctx.advance_token_cursor(word_count);
                        }
                    }
                    let sep = arena.text(sep_text);
                    running = arena.cat(running, sep);
                } else {
                    ip = skip_to_foreach_end(ops, ops_count, ip, &jump_table);
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
                    ip = skip_to_foreach_end(ops, ops_count, ip, &jump_table);
                } else {
                    for_each_stack.push(ForEachState {
                        children,
                        index: 0,
                        body_start: ip + 1,
                        sep_checkpoint: None,
                    });
                }
            }
        }
        ip += 1;
    }

    arena.cat(running, pending)
}

/// Format a child node. Checks for macro verbatim regions first, then
/// recurses into `format_node_inner`.
///
/// Returns `NIL_DOC` if the child is inside an already-consumed macro region
/// (i.e. it should be suppressed). In all cases, the token cursor is advanced
/// past the child's tokens.
fn format_child_doc<'a>(
    ctx: &FmtCtx<'a>,
    child_id: NodeId,
    consumed_regions: &mut u64,
    arena: &mut DocArena<'a>,
) -> DocId {
    let macro_regions = ctx.cursor.macro_regions();
    // Only check macro regions for non-list nodes. List nodes are formatted
    // normally so their individual children can be macro-checked.
    if !macro_regions.is_empty() && ctx.cursor.list_children(child_id, &ctx.dialect).is_none() {
        if let Some(doc) =
            super::formatter::try_macro_verbatim(ctx, macro_regions, arena, consumed_regions)
        {
            // Advance the token cursor through this child's tokens by
            // formatting it (output is discarded).
            let _ = super::formatter::format_node_inner(ctx, child_id, arena, consumed_regions);
            return doc;
        }
    }
    super::formatter::format_node_inner(ctx, child_id, arena, consumed_regions)
}

// ── Comment drain helper ────────────────────────────────────────────────

/// Flush a `DrainResult` into the running/pending accumulators.
///
/// Equivalent to the old `flush_comments()` but operates on single DocIds
/// instead of Vecs, eliminating all Vec allocation.
#[inline]
fn flush_drain(drain: DrainResult, pending: &mut DocId, running: &mut DocId, arena: &mut DocArena) {
    if drain.trailing != NIL_DOC {
        *running = arena.cat(*running, drain.trailing);
    }
    if drain.leading != NIL_DOC {
        // Leading comments already start with a HardLine — drop pending
        // to avoid an extra blank line.
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

/// Pre-compute a jump table mapping ForEachStart/ForEachSelfStart ip → ForEachEnd ip.
fn build_foreach_jump_table(ops: &[u8], ops_count: usize) -> Vec<usize> {
    let mut table = vec![0usize; ops_count];
    let mut stack: Vec<usize> = Vec::new();
    for ip in 0..ops_count {
        match op_at(ops, ip) {
            FmtOp::ForEachStart(_) | FmtOp::ForEachSelfStart => {
                stack.push(ip);
            }
            FmtOp::ForEachEnd => {
                if let Some(start_ip) = stack.pop() {
                    table[start_ip] = ip;
                }
            }
            _ => {}
        }
    }
    table
}

/// Find the matching ForEachEnd for the ForEach starting at `from_ip`.
/// Uses the precomputed jump table for O(1) lookup; falls back to linear scan
/// for ForEachSep (which skips to end mid-body).
fn skip_to_foreach_end(
    ops: &[u8],
    ops_count: usize,
    from_ip: usize,
    jump_table: &[usize],
) -> usize {
    // Direct lookup works for ForEachStart/ForEachSelfStart positions.
    let target = jump_table[from_ip];
    if target != 0 {
        return target;
    }
    // Fallback: linear scan (for ForEachSep which isn't in the jump table).
    let mut depth = 1;
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
/// Previously stored a `Vec<DocId>` (the parent's parts list).
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

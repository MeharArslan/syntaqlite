// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use syntaqlite_syntax::any::{AnyNodeId, AnyParsedStatement, FieldValue, MacroRegion};

use super::comment::{CommentCtx, DrainResult};
use super::doc::{DocArena, DocId, NIL_DOC};
use super::formatter::Formatter;
use crate::dialect::AnyDialect;
use syntaqlite_common::fmt::bytecode::opcodes;

/// Shared context threaded through the iterative formatting traversal.
pub(crate) struct FmtCtx<'a> {
    pub dialect: AnyDialect,
    pub reader: AnyParsedStatement<'a>,
    /// Owned comment context — no lifetime needed since `CommentCtx` owns its data.
    pub comment_ctx: Option<CommentCtx>,
    pub macro_regions: Vec<MacroRegion>,
}

impl<'a> FmtCtx<'a> {
    pub(crate) fn source(&self) -> &'a str {
        self.reader.source()
    }
}

/// Reusable scratch buffers for the iterative interpret loop.
pub(super) struct InterpretScratch {
    group_nest: Vec<GroupNestFrame>,
    calls: Vec<CallFrame>,
    for_each: Vec<ForEachState>,
}

impl InterpretScratch {
    pub(super) fn new() -> Self {
        InterpretScratch {
            group_nest: Vec::new(),
            calls: Vec::new(),
            for_each: Vec::new(),
        }
    }
}

// ── Iterative interpreter ────────────────────────────────────────────────

struct CallFrame {
    ip: usize,
    node_id: AnyNodeId,
    running: DocId,
    pending: DocId,
    gn_save: usize,
    fe_save: usize,
    return_action: ReturnAction,
}

#[derive(PartialEq)]
enum ReturnAction {
    CatOntoRunning,
    /// Wrap child output in parentheses before appending to parent.
    WrapInParens,
    Discard,
}

#[expect(clippy::too_many_lines)]
impl Formatter {
    pub(super) fn interpret_node<'a>(
        &mut self,
        ctx: &FmtCtx<'a>,
        root_id: AnyNodeId,
        arena: &mut DocArena<'a>,
    ) -> DocId {
        self.consumed_regions.clear();
        self.consumed_regions.resize(ctx.macro_regions.len(), false);
        let consumed_regions = &mut self.consumed_regions;
        let scratch = &mut self.interpret_scratch;

        if root_id.is_null() {
            return NIL_DOC;
        }

        let source = ctx.source();
        let Some((tag, fields)) = ctx.reader.extract_fields(root_id) else {
            return NIL_DOC;
        };
        let Some((ops_bytes, ops_len)) = ctx.dialect.fmt_dispatch(tag) else {
            return NIL_DOC;
        };
        scratch.group_nest.clear();
        scratch.calls.clear();
        scratch.for_each.clear();

        let mut cur_node_id: AnyNodeId = root_id;
        let mut ops: &[u8] = &ops_bytes[..ops_len * 6];
        let mut ops_count: usize = ops_len;
        let mut fields = fields;
        let mut running: DocId = NIL_DOC;
        let mut pending: DocId = NIL_DOC;
        let mut gn_save = scratch.group_nest.len();
        let mut fe_save = scratch.for_each.len();
        let mut ip: usize = 0;
        let has_comments = ctx.comment_ctx.is_some();

        macro_rules! push_call_frame {
            ($child_id:expr, $child_ops_bytes:expr, $child_ops_len:expr,
         $child_fields:expr, $return_action_val:expr) => {{
                let frame = CallFrame {
                    ip: ip + 1,
                    node_id: cur_node_id,
                    running,
                    pending,
                    gn_save,
                    fe_save,
                    return_action: $return_action_val,
                };
                scratch.calls.push(frame);

                cur_node_id = $child_id;
                ops = &$child_ops_bytes[..$child_ops_len * 6];
                ops_count = $child_ops_len;
                fields = $child_fields;
                running = NIL_DOC;
                pending = NIL_DOC;
                gn_save = scratch.group_nest.len();
                fe_save = scratch.for_each.len();
                ip = 0;
                continue;
            }};
        }

        loop {
            if ip >= ops_count {
                let result = arena.cat(running, pending);
                scratch.group_nest.truncate(gn_save);
                scratch.for_each.truncate(fe_save);

                if scratch.calls.is_empty() {
                    return result;
                }

                let frame = scratch
                    .calls
                    .pop()
                    .expect("call_stack must contain a parent frame");
                cur_node_id = frame.node_id;
                ip = frame.ip;
                let Some((restored_tag, restored_fields)) = ctx.reader.extract_fields(cur_node_id)
                else {
                    panic!("restored node must resolve to fields");
                };
                let Some((restored_ops, restored_ops_len)) = ctx.dialect.fmt_dispatch(restored_tag)
                else {
                    panic!("restored node must resolve to formatter ops");
                };
                ops = &restored_ops[..restored_ops_len * 6];
                ops_count = restored_ops_len;
                fields = restored_fields;
                running = frame.running;
                pending = frame.pending;
                gn_save = frame.gn_save;
                fe_save = frame.fe_save;

                match frame.return_action {
                    ReturnAction::CatOntoRunning => {
                        running = arena.cat(running, result);
                    }
                    ReturnAction::WrapInParens => {
                        let lp = arena.text("(");
                        let inner = arena.cat(lp, result);
                        let rp = arena.text(")");
                        let wrapped = arena.cat(inner, rp);
                        running = arena.cat(running, wrapped);
                    }
                    ReturnAction::Discard => {}
                }
                continue;
            }

            let op = op_at(ops, ip);
            match op {
                FmtOp::Keyword(sid) => {
                    let kw_text = ctx.dialect.fmt_string(sid);

                    if let Some(ref cctx) = ctx.comment_ctx {
                        if let Some((tok_offset, word_count)) = cctx.peek_keyword_tokens(kw_text) {
                            let drain = cctx.drain_before(tok_offset, source, arena);
                            flush_drain(&drain, &mut pending, &mut running, arena);
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
                    // INVARIANT: Span ops only target Span fields.
                    let FieldValue::Span(s) = fields[idx as usize] else {
                        panic!("Span: field {idx} is not a Span");
                    };

                    if !s.is_empty() {
                        let offset = byte_offset_in(source, s.as_ptr());
                        if let Some(ref cctx) = ctx.comment_ctx {
                            let drain = cctx.drain_before(offset, source, arena);
                            flush_drain(&drain, &mut pending, &mut running, arena);
                            cctx.advance_past(offset + usize_to_u32(s.len()));
                        }
                        let txt = arena.text(s);
                        running = arena.cat(running, txt);
                    }
                }
                FmtOp::Child(idx) => {
                    // INVARIANT: Child ops only target NodeId fields.
                    let FieldValue::NodeId(child_id) = fields[idx as usize] else {
                        panic!("Child: field {idx} is not a NodeId");
                    };

                    if !child_id.is_null() {
                        drain_comments_before_child(
                            ctx.comment_ctx.as_ref(),
                            source,
                            &mut pending,
                            &mut running,
                            arena,
                        );

                        let mut return_action = ReturnAction::CatOntoRunning;
                        let macro_regions = &ctx.macro_regions;
                        if !macro_regions.is_empty()
                            && ctx.reader.list_children(child_id).is_none()
                            && let Some(doc) = super::formatter::try_macro_verbatim(
                                ctx,
                                macro_regions,
                                arena,
                                consumed_regions,
                            )
                        {
                            running = arena.cat(running, doc);
                            return_action = ReturnAction::Discard;
                        }

                        if let Some((ctag, child_fields)) = ctx.reader.extract_fields(child_id)
                            && let Some((child_ops_bytes, child_ops_len)) =
                                ctx.dialect.fmt_dispatch(ctag)
                        {
                            if return_action != ReturnAction::Discard {
                                return_action = child_needs_parens(
                                    ctx,
                                    cur_node_id,
                                    idx,
                                    ctag,
                                    &child_fields,
                                );
                            }
                            push_call_frame!(
                                child_id,
                                child_ops_bytes,
                                child_ops_len,
                                child_fields,
                                return_action
                            );
                        }
                    }
                }
                FmtOp::Line | FmtOp::SoftLine | FmtOp::HardLine => {
                    let doc = match op {
                        FmtOp::Line => arena.line(),
                        FmtOp::SoftLine => arena.softline(),
                        FmtOp::HardLine => arena.hardline(),
                        _ => unreachable!(),
                    };
                    if has_comments {
                        pending = arena.cat(pending, doc);
                    } else {
                        running = arena.cat(running, doc);
                    }
                }
                FmtOp::GroupStart => {
                    scratch.group_nest.push(GroupNestFrame::Group(running));
                    running = NIL_DOC;
                }
                FmtOp::GroupEnd => {
                    running = arena.cat(running, pending);
                    pending = NIL_DOC;
                    let inner = running;
                    match scratch.group_nest.pop().expect("unmatched GroupEnd") {
                        GroupNestFrame::Group(parent) => {
                            let g = arena.group(inner);
                            running = arena.cat(parent, g);
                        }
                        GroupNestFrame::Nest(..) => panic!("expected Group frame"),
                    }
                }
                FmtOp::NestStart(indent) => {
                    scratch
                        .group_nest
                        .push(GroupNestFrame::Nest(indent, running));
                    running = NIL_DOC;
                }
                FmtOp::NestEnd => {
                    running = arena.cat(running, pending);
                    pending = NIL_DOC;
                    let inner = running;
                    match scratch.group_nest.pop().expect("unmatched NestEnd") {
                        GroupNestFrame::Nest(indent, parent) => {
                            let n = arena.nest(indent, inner);
                            running = arena.cat(parent, n);
                        }
                        GroupNestFrame::Group(_) => panic!("expected Nest frame"),
                    }
                }
                FmtOp::IfSet(idx, skip) => {
                    // INVARIANT: IfSet ops only target NodeId fields.
                    let FieldValue::NodeId(id) = fields[idx as usize] else {
                        panic!("IfSet: field {idx} is not a NodeId");
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
                    // INVARIANT: ForEachStart ops only target NodeId fields.
                    let FieldValue::NodeId(list_id) = fields[idx as usize] else {
                        panic!("ForEachStart: field {idx} is not a NodeId");
                    };
                    if list_id.is_null() {
                        ip = skip_to_foreach_end(ops, ops_count, ip);
                    } else {
                        let children: &'a [AnyNodeId] =
                            ctx.reader.list_children(list_id).unwrap_or(&[]);
                        if children.is_empty() {
                            ip = skip_to_foreach_end(ops, ops_count, ip);
                        } else {
                            scratch.for_each.push(ForEachState {
                                list_id,
                                index: 0,
                                body_start: ip + 1,
                                sep_checkpoint: None,
                            });
                        }
                    }
                }
                FmtOp::ChildItem => {
                    let state = scratch.for_each.last().expect("ChildItem outside ForEach");
                    let children = ctx.reader.list_children(state.list_id).unwrap_or(&[]);
                    let child_id = children[state.index];

                    let macro_regions = &ctx.macro_regions;
                    let macro_doc = if !macro_regions.is_empty()
                        && ctx.reader.list_children(child_id).is_none()
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
                        let state = scratch
                            .for_each
                            .last_mut()
                            .expect("ForEachState must exist while handling ChildItem");
                        if let Some((saved_running, saved_pending)) = state.sep_checkpoint.take() {
                            running = saved_running;
                            pending = saved_pending;
                        }
                        return_action = ReturnAction::Discard;
                    } else {
                        drain_comments_before_child(
                            ctx.comment_ctx.as_ref(),
                            source,
                            &mut pending,
                            &mut running,
                            arena,
                        );

                        if let Some(verbatim) = macro_doc {
                            running = arena.cat(running, verbatim);
                            return_action = ReturnAction::Discard;
                        } else {
                            return_action = ReturnAction::CatOntoRunning;
                        }
                    }

                    if let Some((ctag, child_fields)) = ctx.reader.extract_fields(child_id)
                        && let Some((child_ops_bytes, child_ops_len)) =
                            ctx.dialect.fmt_dispatch(ctag)
                    {
                        push_call_frame!(
                            child_id,
                            child_ops_bytes,
                            child_ops_len,
                            child_fields,
                            return_action
                        );
                    }
                }
                FmtOp::ForEachSep(sid) => {
                    let state = scratch
                        .for_each
                        .last_mut()
                        .expect("ForEachSep outside ForEach");
                    let children = ctx.reader.list_children(state.list_id).unwrap_or(&[]);
                    if state.index < children.len() - 1 {
                        state.sep_checkpoint = Some((running, pending));
                        let sep_text = ctx.dialect.fmt_string(sid);
                        if let Some(ref cctx) = ctx.comment_ctx
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
                    let state = scratch
                        .for_each
                        .last_mut()
                        .expect("ForEachEnd outside ForEach");
                    state.index += 1;
                    let children = ctx.reader.list_children(state.list_id).unwrap_or(&[]);
                    if state.index < children.len() {
                        ip = state.body_start;
                        continue;
                    }
                    scratch.for_each.pop();
                }
                FmtOp::IfBool(idx, skip) => {
                    // INVARIANT: IfBool ops only target Bool fields.
                    let FieldValue::Bool(val) = fields[idx as usize] else {
                        panic!("IfBool: field {idx} is not a Bool");
                    };
                    if !val {
                        ip += skip as usize;
                    }
                }
                FmtOp::IfFlag(idx, mask, skip) => {
                    // INVARIANT: IfFlag ops only target Flags fields.
                    let FieldValue::Flags(f) = fields[idx as usize] else {
                        panic!("IfFlag: field {idx} is not Flags");
                    };
                    if f & mask == 0 {
                        ip += skip as usize;
                    }
                }
                FmtOp::IfEnum(idx, ordinal, skip) => {
                    // INVARIANT: IfEnum ops only target Enum fields.
                    let FieldValue::Enum(val) = fields[idx as usize] else {
                        panic!("IfEnum: field {idx} is not an Enum");
                    };
                    if val != u32::from(ordinal) {
                        ip += skip as usize;
                    }
                }
                FmtOp::IfSpan(idx, skip) => {
                    // INVARIANT: IfSpan ops only target Span fields.
                    let FieldValue::Span(s) = fields[idx as usize] else {
                        panic!("IfSpan: field {idx} is not a Span");
                    };
                    if s.is_empty() {
                        ip += skip as usize;
                    }
                }
                FmtOp::EnumDisplay(idx, base) => {
                    // INVARIANT: EnumDisplay ops only target Enum fields.
                    let FieldValue::Enum(ordinal) = fields[idx as usize] else {
                        panic!("EnumDisplay: field {idx} is not an Enum");
                    };
                    let string_id = ctx
                        .dialect
                        .fmt_enum_display_val(base as usize + ordinal as usize);
                    let kw_text = ctx.dialect.fmt_string(string_id);
                    if let Some(ref cctx) = ctx.comment_ctx {
                        if let Some((tok_offset, word_count)) = cctx.peek_keyword_tokens(kw_text) {
                            let drain = cctx.drain_before(tok_offset, source, arena);
                            flush_drain(&drain, &mut pending, &mut running, arena);
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
                    let children = ctx
                        .reader
                        .list_children(cur_node_id)
                        .expect("ForEachSelfStart on non-list node");
                    if children.is_empty() {
                        ip = skip_to_foreach_end(ops, ops_count, ip);
                    } else {
                        scratch.for_each.push(ForEachState {
                            list_id: cur_node_id,
                            index: 0,
                            body_start: ip + 1,
                            sep_checkpoint: None,
                        });
                    }
                }
            }
            ip += 1;
        }
    }
}

// ── Comment drain helpers ───────────────────────────────────────────────

#[inline]
fn flush_drain(
    drain: &DrainResult,
    pending: &mut DocId,
    running: &mut DocId,
    arena: &mut DocArena,
) {
    if drain.trailing != NIL_DOC {
        *running = arena.cat(*running, drain.trailing);
    }
    if drain.leading == NIL_DOC {
        *running = arena.cat(*running, *pending);
        *pending = NIL_DOC;
    } else {
        *pending = NIL_DOC;
        *running = arena.cat(*running, drain.leading);
    }
}

#[inline]
fn drain_comments_before_child<'a>(
    comment_ctx: Option<&CommentCtx>,
    source: &'a str,
    pending: &mut DocId,
    running: &mut DocId,
    arena: &mut DocArena<'a>,
) {
    if let Some(cctx) = comment_ctx {
        if let Some((offset, _)) = cctx.peek_next_token() {
            let drain = cctx.drain_before(offset, source, arena);
            flush_drain(&drain, pending, running, arena);
        } else {
            *running = arena.cat(*running, *pending);
            *pending = NIL_DOC;
        }
    }
}

// ── Bytecode helpers ────────────────────────────────────────────────────

#[inline]
fn op_at(ops: &[u8], ip: usize) -> FmtOp {
    let base = ip * 6;
    let opcode = ops[base];
    let a = ops[base + 1];
    let b = u16::from_le_bytes([ops[base + 2], ops[base + 3]]);
    let c = u16::from_le_bytes([ops[base + 4], ops[base + 5]]);
    FmtOp::decode(opcode, a, b, c)
}

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

#[derive(Debug, Clone, Copy, PartialEq)]
enum FmtOp {
    Keyword(StringId),
    Span(FieldIdx),
    Child(FieldIdx),
    Line,
    SoftLine,
    HardLine,
    GroupStart,
    GroupEnd,
    NestStart(i16),
    NestEnd,
    IfSet(FieldIdx, SkipCount),
    Else(SkipCount),
    EndIf,
    ForEachStart(FieldIdx),
    ChildItem,
    ForEachSep(StringId),
    ForEachEnd,
    IfBool(FieldIdx, SkipCount),
    IfFlag(FieldIdx, u8, SkipCount),
    IfEnum(FieldIdx, u16, SkipCount),
    IfSpan(FieldIdx, SkipCount),
    EnumDisplay(FieldIdx, u16),
    ForEachSelfStart,
}

impl FmtOp {
    #[inline]
    pub(crate) fn decode(opcode: u8, a: u8, b: u16, c: u16) -> Self {
        match opcode {
            opcodes::KEYWORD => FmtOp::Keyword(b),
            opcodes::SPAN => FmtOp::Span(a.into()),
            opcodes::CHILD => FmtOp::Child(a.into()),
            opcodes::LINE => FmtOp::Line,
            opcodes::SOFTLINE => FmtOp::SoftLine,
            opcodes::HARDLINE => FmtOp::HardLine,
            opcodes::GROUP_START => FmtOp::GroupStart,
            opcodes::GROUP_END => FmtOp::GroupEnd,
            opcodes::NEST_START => FmtOp::NestStart(i16::from_le_bytes(b.to_le_bytes())),
            opcodes::NEST_END => FmtOp::NestEnd,
            opcodes::IF_SET => FmtOp::IfSet(a.into(), c),
            opcodes::ELSE_OP => FmtOp::Else(c),
            opcodes::END_IF => FmtOp::EndIf,
            opcodes::FOR_EACH_START => FmtOp::ForEachStart(a.into()),
            opcodes::CHILD_ITEM => FmtOp::ChildItem,
            opcodes::FOR_EACH_SEP => FmtOp::ForEachSep(b),
            opcodes::FOR_EACH_END => FmtOp::ForEachEnd,
            opcodes::IF_BOOL => FmtOp::IfBool(a.into(), c),
            opcodes::IF_FLAG => FmtOp::IfFlag(
                a.into(),
                u8::try_from(b).expect("IF_FLAG mask must fit in u8"),
                c,
            ),
            opcodes::IF_ENUM => FmtOp::IfEnum(a.into(), b, c),
            opcodes::IF_SPAN => FmtOp::IfSpan(a.into(), c),
            opcodes::ENUM_DISPLAY => FmtOp::EnumDisplay(a.into(), b),
            opcodes::FOR_EACH_SELF_START => FmtOp::ForEachSelfStart,
            _ => panic!("unknown opcode in fmt data"),
        }
    }
}

#[inline]
fn usize_to_u32(value: usize) -> u32 {
    u32::try_from(value).expect("value must fit in u32")
}

#[inline]
fn byte_offset_in(source: &str, ptr: *const u8) -> u32 {
    let base = source.as_ptr() as usize;
    let start = ptr as usize;
    let offset = start
        .checked_sub(base)
        .expect("span pointer must be within source");
    usize_to_u32(offset)
}

enum GroupNestFrame {
    Group(DocId),
    Nest(i16, DocId),
}

// ── Expression parenthesization helpers ─────────────────────────────────

/// `SQLite` binary operator precedence (higher = binds tighter).
///
/// Must match the Lemon grammar precedence declarations in
/// `syntaqlite-syntax/parser-actions/_common.y` (lines 149-161):
///
/// ```text
/// %left OR.                                           // prec 1
/// %left AND.                                          // prec 2
/// %left IS MATCH LIKE_KW BETWEEN IN ... NE EQ.       // prec 3 (EQ, NE only for BinaryOp)
/// %left GT LE LT GE.                                  // prec 4
/// %right ESCAPE.                                       // (not a BinaryOp)
/// %left BITAND BITOR LSHIFT RSHIFT.                   // prec 5
/// %left PLUS MINUS.                                    // prec 6
/// %left STAR SLASH REM.                                // prec 7
/// %left CONCAT PTR.                                    // prec 8
/// ```
fn binary_op_precedence(op_ordinal: u32) -> Option<u8> {
    // BinaryOp enum order from common.synq:
    //   PLUS=0, MINUS=1, STAR=2, SLASH=3, REM=4,
    //   LT=5, GT=6, LE=7, GE=8, EQ=9, NE=10,
    //   AND=11, OR=12, BIT_AND=13, BIT_OR=14,
    //   LSHIFT=15, RSHIFT=16, CONCAT=17, PTR=18
    match op_ordinal {
        12 => Some(1),              // OR
        11 => Some(2),              // AND
        9 | 10 => Some(3),          // EQ, NE
        5..=8 => Some(4),           // LT, GT, LE, GE
        13..=16 => Some(5),         // BIT_AND, BIT_OR, LSHIFT, RSHIFT
        0 | 1 => Some(6),           // PLUS, MINUS
        2..=4 => Some(7),           // STAR, SLASH, REM
        17 | 18 => Some(8),         // CONCAT, PTR
        _ => None,
    }
}

/// Determine whether a child node needs to be wrapped in parentheses.
///
/// This handles two cases:
/// 1. **`BinaryExpr` precedence**: when a `BinaryExpr` child of a `BinaryExpr` parent
///    has lower (or equal, for right-child) precedence, parens are needed to
///    preserve the original grouping.
/// 2. **List-as-expression**: when a list node (`ExprList`) appears in a position
///    where the parent doesn't already wrap it in explicit parens (e.g., as a
///    row value in SELECT or as a SET clause value), parens are needed.
fn child_needs_parens(
    ctx: &FmtCtx,
    parent_id: AnyNodeId,
    child_field_idx: u16,
    child_tag: syntaqlite_syntax::any::AnyNodeTag,
    child_fields: &syntaqlite_syntax::any::NodeFields,
) -> ReturnAction {
    let grammar = &*ctx.dialect;

    // Case 1: BinaryExpr precedence
    let parent_tag = ctx.reader.extract_fields(parent_id).map(|(t, _)| t);
    if let Some(ptag) = parent_tag {
        let parent_name = grammar.node_name(ptag);
        let child_name = grammar.node_name(child_tag);
        if parent_name == "BinaryExpr" && child_name == "BinaryExpr" {
            // BinaryExpr fields: [op, left, right]
            // op is field 0 (Enum), left is field 1 (NodeId), right is field 2 (NodeId)
            let parent_fields = ctx.reader.extract_fields(parent_id);
            if let Some((_, pf)) = parent_fields
                && let (FieldValue::Enum(parent_op), FieldValue::Enum(child_op)) =
                    (pf[0], child_fields[0])
                    && let (Some(parent_prec), Some(child_prec)) = (
                        binary_op_precedence(parent_op),
                        binary_op_precedence(child_op),
                    ) {
                        // child_field_idx: 1 = left child, 2 = right child
                        let is_right_child = child_field_idx == 2;
                        if child_prec < parent_prec || (is_right_child && child_prec == parent_prec)
                        {
                            return ReturnAction::WrapInParens;
                        }
                    }
        }

        // Case 2: UnaryExpr wrapping BinaryExpr (e.g., NOT (a AND b))
        if parent_name == "UnaryExpr" && child_name == "BinaryExpr" {
            // UnaryExpr fields: [op, operand]
            // op is field 0 (Enum), operand is field 1 (NodeId)
            // UnaryOp: MINUS=0, PLUS=1, BIT_NOT=2, NOT=3
            let parent_fields = ctx.reader.extract_fields(parent_id);
            if let Some((_, pf)) = parent_fields
                && let FieldValue::Enum(unary_op) = pf[0] {
                    // NOT has lower precedence than all binary operators
                    if unary_op == 3 {
                        return ReturnAction::WrapInParens;
                    }
                }
        }
    }

    // Case 3: ExprList appearing as an expression (row value or multi-column SET value).
    // ExprList needs parens when it appears in a field typed as Expr in .synq
    // (i.e., a single-expression context). We detect this by checking the parent
    // node/field name rather than scanning bytecode, because bytecode layout is
    // unreliable with conditional blocks.
    if let Some(ptag) = parent_tag
        && grammar.is_list(child_tag) && grammar.node_name(child_tag) == "ExprList" {
            let field_name = grammar
                .field_meta(ptag)
                .nth(child_field_idx as usize)
                .map(|m| m.name());
            // Fields typed as `index Expr` that may contain ExprList as a row value.
            // Fields like `args`, `columns`, `groupby`, `ref_columns`, etc. are typed
            // as `index ExprList` and the parent already provides any needed wrapping.
            if matches!(
                field_name,
                Some("expr" | "value" | "left" | "right" | "operand")
            ) {
                return ReturnAction::WrapInParens;
            }
        }

    ReturnAction::CatOntoRunning
}

struct ForEachState {
    list_id: AnyNodeId,
    index: usize,
    body_start: usize,
    sep_checkpoint: Option<(DocId, DocId)>,
}

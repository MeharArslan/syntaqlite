// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use crate::dialect::Dialect;
use crate::parser::{FieldVal, NodeId};

use super::bytecode::opcodes;
use super::doc::{DocArena, DocId};
use super::comment::{flush_comments, CommentCtx};

// ── Interpreter (pub(crate)) ────────────────────────────────────────────

/// Bytecode interpreter for formatting ops. Reads string/enum data via
/// `Dialect` safe accessors and decodes ops from a byte slice on demand.
pub(crate) struct Interpreter<'a, 'b> {
    dialect: Dialect<'a>,
    // Raw ops for the current node (6 bytes each, decoded on demand)
    ops: &'b [u8],
    ops_len: usize,
    // Callbacks
    format_child: &'b dyn Fn(NodeId, &mut DocArena<'a>) -> DocId,
    resolve_list: &'b dyn Fn(NodeId) -> Vec<NodeId>,
    comment_ctx: Option<&'b CommentCtx<'a>>,
    source_offset: Option<&'b dyn Fn(NodeId) -> Option<u32>>,
}

impl<'a, 'b> Interpreter<'a, 'b> {
    pub fn new(
        dialect: Dialect<'a>,
        ops: &'b [u8],
        ops_len: usize,
        format_child: &'b dyn Fn(NodeId, &mut DocArena<'a>) -> DocId,
        resolve_list: &'b dyn Fn(NodeId) -> Vec<NodeId>,
        comment_ctx: Option<&'b CommentCtx<'a>>,
        source_offset: Option<&'b dyn Fn(NodeId) -> Option<u32>>,
    ) -> Self {
        Interpreter {
            dialect,
            ops,
            ops_len,
            format_child,
            resolve_list,
            comment_ctx,
            source_offset,
        }
    }

    /// Interpret the ops into a Doc tree.
    ///
    /// `fields` contains typed values extracted from the node struct.
    /// `list_children` is set when the current node is a list (for `ForEachSelfStart`).
    pub fn run(
        &self,
        fields: &[FieldVal<'a>],
        list_children: Option<&[NodeId]>,
        arena: &mut DocArena<'a>,
    ) -> DocId {
        let mut parts: Vec<DocId> = Vec::new();
        let mut stack: Vec<StackFrame> = Vec::new();
        let mut for_each_stack: Vec<ForEachState> = Vec::new();
        let mut pending_lines: Vec<DocId> = Vec::new();
        let mut ip: usize = 0;
        let has_comments = self.comment_ctx.is_some();

        while ip < self.ops_len {
            match self.op_at(ip) {
                FmtOp::Keyword(sid) => {
                    if has_comments {
                        parts.extend(pending_lines.drain(..));
                    }
                    parts.push(arena.keyword(self.string(sid)));
                }
                FmtOp::Span(idx) => {
                    let FieldVal::Span(s, offset) = fields[idx as usize] else {
                        panic!("Span: field {} is not a Span", idx);
                    };
                    if !s.is_empty() {
                        if let Some(tc) = self.comment_ctx {
                            let drain = tc.drain_before(offset, arena);
                            flush_comments(drain, &mut pending_lines, &mut parts);
                            tc.set_source_end(offset + s.len() as u32);
                        }
                        parts.push(arena.text(s));
                    }
                }
                FmtOp::Child(idx) => {
                    let FieldVal::NodeId(child_id) = fields[idx as usize] else {
                        panic!("Child: field {} is not a NodeId", idx);
                    };
                    if !child_id.is_null() {
                        if let (Some(tc), Some(so)) = (self.comment_ctx, self.source_offset) {
                            if let Some(offset) = so(child_id) {
                                let drain = tc.drain_before(offset, arena);
                                flush_comments(drain, &mut pending_lines, &mut parts);
                            } else {
                                parts.extend(pending_lines.drain(..));
                            }
                        }
                        parts.push((self.format_child)(child_id, arena));
                    }
                }
                FmtOp::Line => {
                    if has_comments {
                        pending_lines.push(arena.line());
                    } else {
                        parts.push(arena.line());
                    }
                }
                FmtOp::SoftLine => {
                    if has_comments {
                        pending_lines.push(arena.softline());
                    } else {
                        parts.push(arena.softline());
                    }
                }
                FmtOp::HardLine => {
                    if has_comments {
                        pending_lines.push(arena.hardline());
                    } else {
                        parts.push(arena.hardline());
                    }
                }
                FmtOp::GroupStart => {
                    stack.push(StackFrame::Group(std::mem::take(&mut parts)));
                }
                FmtOp::GroupEnd => {
                    // Flush any pending lines before closing the group.
                    parts.extend(pending_lines.drain(..));
                    let inner = arena.cats(&parts);
                    match stack.pop().expect("unmatched GroupEnd") {
                        StackFrame::Group(parent) => {
                            parts = parent;
                            parts.push(arena.group(inner));
                        }
                        _ => panic!("expected Group frame"),
                    }
                }
                FmtOp::NestStart(indent) => {
                    stack.push(StackFrame::Nest(indent, std::mem::take(&mut parts)));
                }
                FmtOp::NestEnd => {
                    parts.extend(pending_lines.drain(..));
                    let inner = arena.cats(&parts);
                    match stack.pop().expect("unmatched NestEnd") {
                        StackFrame::Nest(indent, parent) => {
                            parts = parent;
                            parts.push(arena.nest(indent, inner));
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
                    } else if let (Some(tc), Some(so)) = (self.comment_ctx, self.source_offset) {
                        // Drain comments before this clause's source range.
                        if let Some(offset) = so(id) {
                            let drain = tc.drain_before(offset, arena);
                            flush_comments(drain, &mut pending_lines, &mut parts);
                        }
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
                        ip = self.skip_to_foreach_end(ip);
                    } else {
                        let children = (self.resolve_list)(list_id);
                        if children.is_empty() {
                            ip = self.skip_to_foreach_end(ip);
                        } else {
                            for_each_stack.push(ForEachState {
                                children,
                                index: 0,
                                body_start: ip + 1,
                            });
                        }
                    }
                }
                FmtOp::ChildItem => {
                    let state = for_each_stack.last().expect("ChildItem outside ForEach");
                    let child_id = state.children[state.index];
                    if let (Some(tc), Some(so)) = (self.comment_ctx, self.source_offset) {
                        if let Some(offset) = so(child_id) {
                            let drain = tc.drain_before(offset, arena);
                            flush_comments(drain, &mut pending_lines, &mut parts);
                        } else {
                            parts.extend(pending_lines.drain(..));
                        }
                    }
                    parts.push((self.format_child)(child_id, arena));
                }
                FmtOp::ForEachSep(sid) => {
                    let state = for_each_stack.last().expect("ForEachSep outside ForEach");
                    if state.index < state.children.len() - 1 {
                        parts.push(arena.text(self.string(sid)));
                    } else {
                        ip = self.skip_to_foreach_end(ip);
                        continue;
                    }
                }
                FmtOp::ForEachEnd => {
                    let state = for_each_stack.last_mut().expect("ForEachEnd outside ForEach");
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
                    if has_comments {
                        parts.extend(pending_lines.drain(..));
                    }
                    let string_id = self.enum_display_val(base as usize + ordinal as usize);
                    parts.push(arena.keyword(self.string(string_id)));
                }
                FmtOp::ForEachSelfStart => {
                    let children = list_children.expect("ForEachSelfStart on non-list node");
                    if children.is_empty() {
                        ip = self.skip_to_foreach_end(ip);
                    } else {
                        for_each_stack.push(ForEachState {
                            children: children.to_vec(),
                            index: 0,
                            body_start: ip + 1,
                        });
                    }
                }
            }
            ip += 1;
        }

        // Flush any remaining pending lines.
        parts.extend(pending_lines.drain(..));

        arena.cats(&parts)
    }

    /// Look up a string from the C string table by index.
    #[inline]
    fn string(&self, sid: u16) -> &'a str {
        self.dialect.fmt_string(sid)
    }

    /// Look up a value in the enum display table.
    #[inline]
    fn enum_display_val(&self, idx: usize) -> u16 {
        self.dialect.fmt_enum_display_val(idx)
    }

    /// Decode the op at position `ip` from the raw byte stream.
    #[inline(always)]
    fn op_at(&self, ip: usize) -> FmtOp {
        debug_assert!(ip < self.ops_len);
        let base = ip * 6;
        let opcode = self.ops[base];
        let a = self.ops[base + 1];
        let b = u16::from_le_bytes([self.ops[base + 2], self.ops[base + 3]]);
        let c = u16::from_le_bytes([self.ops[base + 4], self.ops[base + 5]]);
        FmtOp::decode(opcode, a, b, c)
    }

    /// Find the matching ForEachEnd scanning forward from `from_ip`.
    fn skip_to_foreach_end(&self, from_ip: usize) -> usize {
        let mut depth = 1;
        let mut ip = from_ip + 1;
        while ip < self.ops_len {
            match self.op_at(ip) {
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

#[cfg(test)]
impl FmtOp {
    /// Encode this op into its 6-byte binary representation.
    const fn encode(self) -> [u8; 6] {
        let (opcode, a, b, c): (u8, u8, u16, u16) = match self {
            FmtOp::Keyword(b) => (opcodes::KEYWORD, 0, b, 0),
            FmtOp::Span(a) => (opcodes::SPAN, a as u8, 0, 0),
            FmtOp::Child(a) => (opcodes::CHILD, a as u8, 0, 0),
            FmtOp::Line => (opcodes::LINE, 0, 0, 0),
            FmtOp::SoftLine => (opcodes::SOFTLINE, 0, 0, 0),
            FmtOp::HardLine => (opcodes::HARDLINE, 0, 0, 0),
            FmtOp::GroupStart => (opcodes::GROUP_START, 0, 0, 0),
            FmtOp::GroupEnd => (opcodes::GROUP_END, 0, 0, 0),
            FmtOp::NestStart(indent) => (opcodes::NEST_START, 0, indent as u16, 0),
            FmtOp::NestEnd => (opcodes::NEST_END, 0, 0, 0),
            FmtOp::IfSet(a, c) => (opcodes::IF_SET, a as u8, 0, c),
            FmtOp::Else(c) => (opcodes::ELSE_OP, 0, 0, c),
            FmtOp::EndIf => (opcodes::END_IF, 0, 0, 0),
            FmtOp::ForEachStart(a) => (opcodes::FOR_EACH_START, a as u8, 0, 0),
            FmtOp::ChildItem => (opcodes::CHILD_ITEM, 0, 0, 0),
            FmtOp::ForEachSep(b) => (opcodes::FOR_EACH_SEP, 0, b, 0),
            FmtOp::ForEachEnd => (opcodes::FOR_EACH_END, 0, 0, 0),
            FmtOp::IfBool(a, c) => (opcodes::IF_BOOL, a as u8, 0, c),
            FmtOp::IfFlag(a, mask, c) => (opcodes::IF_FLAG, a as u8, mask as u16, c),
            FmtOp::IfEnum(a, b, c) => (opcodes::IF_ENUM, a as u8, b, c),
            FmtOp::IfSpan(a, c) => (opcodes::IF_SPAN, a as u8, 0, c),
            FmtOp::EnumDisplay(a, b) => (opcodes::ENUM_DISPLAY, a as u8, b, 0),
            FmtOp::ForEachSelfStart => (opcodes::FOR_EACH_SELF_START, 0, 0, 0),
        };
        let bb = b.to_le_bytes();
        let cb = c.to_le_bytes();
        [opcode, a, bb[0], bb[1], cb[0], cb[1]]
    }
}

enum StackFrame {
    Group(Vec<DocId>),
    Nest(i16, Vec<DocId>),
}

struct ForEachState {
    children: Vec<NodeId>,
    index: usize,
    body_start: usize,
}

// ── Test helpers ────────────────────────────────────────────────────────

/// Encode a slice of `FmtOp`s into raw bytes.
#[cfg(test)]
fn encode_ops(ops: &[FmtOp]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(ops.len() * 6);
    for op in ops {
        bytes.extend_from_slice(&op.encode());
    }
    bytes
}

/// Test helper: owns string table data and a minimal `ffi::Dialect`.
#[cfg(test)]
pub(crate) struct TestDialect {
    _strings: Vec<std::ffi::CString>,
    _ptrs: Vec<*const std::ffi::c_char>,
    _enum_display: Vec<u16>,
    raw: crate::dialect::ffi::Dialect,
}

#[cfg(test)]
impl TestDialect {
    pub fn new(strings: &[&str], enum_display: &[u16]) -> Self {
        let cstrings: Vec<std::ffi::CString> = strings
            .iter()
            .map(|s| std::ffi::CString::new(*s).unwrap())
            .collect();
        let ptrs: Vec<*const std::ffi::c_char> =
            cstrings.iter().map(|cs| cs.as_ptr()).collect();
        let enum_display = enum_display.to_vec();

        let raw = crate::dialect::ffi::Dialect {
            name: std::ptr::null(),
            tables: std::ptr::null(),
            reduce_actions: std::ptr::null(),
            range_meta: std::ptr::null(),
            tk_space: 0,
            tk_semi: 0,
            tk_comment: 0,
            node_count: 0,
            node_names: std::ptr::null(),
            field_meta: std::ptr::null(),
            field_meta_counts: std::ptr::null(),
            list_tags: std::ptr::null(),
            fmt_strings: ptrs.as_ptr(),
            fmt_string_count: ptrs.len() as u16,
            fmt_enum_display: enum_display.as_ptr(),
            fmt_enum_display_count: enum_display.len() as u16,
            fmt_ops: std::ptr::null(),
            fmt_op_count: 0,
            fmt_dispatch: std::ptr::null(),
            fmt_dispatch_count: 0,
        };

        TestDialect {
            _strings: cstrings,
            _ptrs: ptrs,
            _enum_display: enum_display,
            raw,
        }
    }

    pub fn dialect(&self) -> Dialect<'_> {
        Dialect { raw: &self.raw }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::NodeId;
    use crate::parser::FieldVal;
    use super::super::FormatConfig;
    use super::super::doc::{DocArena, NIL_DOC};
    use super::super::render::render;

    fn noop_child(_: NodeId, _: &mut DocArena) -> u32 {
        NIL_DOC
    }

    fn no_lists(_: NodeId) -> Vec<NodeId> {
        panic!("resolve_list not expected")
    }

    fn run(ops: &[FmtOp], strings: &[&str], enum_display: &[u16], fields: &[FieldVal], config: &FormatConfig) -> String {
        let td = TestDialect::new(strings, enum_display);
        let dialect = td.dialect();
        let ops_bytes = encode_ops(ops);
        let mut arena = DocArena::new();
        let interp = Interpreter::new(
            dialect, &ops_bytes, ops.len(),
            &noop_child, &no_lists, None, None,
        );
        let doc = interp.run(fields, None, &mut arena);
        render(&arena, doc, config)
    }

    fn run_default(ops: &[FmtOp], strings: &[&str], fields: &[FieldVal]) -> String {
        run(ops, strings, &[], fields, &FormatConfig::default())
    }

    const NULL: NodeId = NodeId::NULL;

    #[test]
    fn single_keyword() {
        assert_eq!(run_default(&[FmtOp::Keyword(0)], &["SELECT"], &[]), "SELECT");
    }

    #[test]
    fn group_fits_flat() {
        let ops = &[
            FmtOp::GroupStart,
            FmtOp::Keyword(0),
            FmtOp::Line,
            FmtOp::Keyword(1),
            FmtOp::GroupEnd,
        ];
        assert_eq!(run_default(ops, &["SELECT", "FROM"], &[]), "SELECT FROM");
    }

    #[test]
    fn group_breaks_when_narrow() {
        let ops = &[
            FmtOp::GroupStart,
            FmtOp::Keyword(0),
            FmtOp::Line,
            FmtOp::Keyword(1),
            FmtOp::GroupEnd,
        ];
        let config = FormatConfig { line_width: 5, ..Default::default() };
        assert_eq!(run(ops, &["SELECT", "FROM"], &[], &[], &config), "SELECT\nFROM");
    }

    #[test]
    fn nest_indentation() {
        let ops = &[
            FmtOp::GroupStart,
            FmtOp::Keyword(0),
            FmtOp::NestStart(4),
            FmtOp::Line,
            FmtOp::Keyword(1),
            FmtOp::NestEnd,
            FmtOp::GroupEnd,
        ];
        let config = FormatConfig { line_width: 5, ..Default::default() };
        assert_eq!(run(ops, &["SELECT", "a"], &[], &[], &config), "SELECT\n    a");
    }

    #[test]
    fn span_reads_source_text() {
        let fields = &[FieldVal::Span("hello", 0)];
        let ops = &[FmtOp::Span(0)];
        assert_eq!(run_default(ops, &[], fields), "hello");
    }

    #[test]
    fn child_recurses_into_child_node() {
        let td = TestDialect::new(&[], &[]);
        let dialect = td.dialect();
        let ops_bytes = encode_ops(&[FmtOp::Child(0)]);
        let fields = &[FieldVal::NodeId(NodeId(42))];
        let mut arena = DocArena::new();
        let format_child = |node_id: NodeId, arena: &mut DocArena| {
            assert_eq!(node_id, NodeId(42));
            arena.text("child_result")
        };
        let interp = Interpreter::new(
            dialect, &ops_bytes, 1,
            &format_child, &no_lists, None, None,
        );
        let doc = interp.run(fields, None, &mut arena);
        assert_eq!(render(&arena, doc, &FormatConfig::default()), "child_result");
    }

    #[test]
    fn child_skips_null_node() {
        let fields = &[FieldVal::NodeId(NULL)];
        let ops = &[FmtOp::Keyword(0), FmtOp::Child(0), FmtOp::Keyword(1)];
        assert_eq!(run_default(ops, &["a", "b"], fields), "ab");
    }

    #[test]
    fn ifset_executes_then_branch() {
        let fields = &[FieldVal::NodeId(NodeId(42))];
        let ops = &[
            FmtOp::IfSet(0, 2),
            FmtOp::Keyword(0),
            FmtOp::Else(1),
            FmtOp::Keyword(1),
            FmtOp::EndIf,
        ];
        assert_eq!(run_default(ops, &["YES", "NO"], fields), "YES");
    }

    #[test]
    fn ifset_executes_else_branch() {
        let fields = &[FieldVal::NodeId(NULL)];
        let ops = &[
            FmtOp::IfSet(0, 2),
            FmtOp::Keyword(0),
            FmtOp::Else(1),
            FmtOp::Keyword(1),
            FmtOp::EndIf,
        ];
        assert_eq!(run_default(ops, &["YES", "NO"], fields), "NO");
    }

    #[test]
    fn ifset_without_else() {
        let fields = &[FieldVal::NodeId(NULL)];
        let ops = &[
            FmtOp::Keyword(0),
            FmtOp::IfSet(0, 2),
            FmtOp::Line,
            FmtOp::Keyword(1),
            FmtOp::EndIf,
            FmtOp::Keyword(2),
        ];
        assert_eq!(run_default(ops, &["a", "b", "c"], fields), "ac");
    }

    #[test]
    fn foreach_comma_separated() {
        let td = TestDialect::new(&[", "], &[]);
        let dialect = td.dialect();
        let ops = &[
            FmtOp::GroupStart,
            FmtOp::ForEachStart(0),
            FmtOp::ChildItem,
            FmtOp::ForEachSep(0),
            FmtOp::ForEachEnd,
            FmtOp::GroupEnd,
        ];
        let ops_bytes = encode_ops(ops);
        let fields = &[FieldVal::NodeId(NodeId(99))];
        let format_child = |id: NodeId, arena: &mut DocArena| match id.0 {
            10 => arena.text("a"),
            20 => arena.text("b"),
            30 => arena.text("c"),
            _ => panic!("unexpected"),
        };
        let resolve_list = |id: NodeId| match id.0 {
            99 => vec![NodeId(10), NodeId(20), NodeId(30)],
            _ => panic!("unexpected list"),
        };
        let mut arena = DocArena::new();
        let interp = Interpreter::new(
            dialect, &ops_bytes, ops.len(),
            &format_child, &resolve_list, None, None,
        );
        let doc = interp.run(fields, None, &mut arena);
        assert_eq!(render(&arena, doc, &FormatConfig::default()), "a, b, c");
    }

    #[test]
    fn foreach_with_line_breaks() {
        let td = TestDialect::new(&[","], &[]);
        let dialect = td.dialect();
        let ops = &[
            FmtOp::GroupStart,
            FmtOp::ForEachStart(0),
            FmtOp::ChildItem,
            FmtOp::ForEachSep(0),
            FmtOp::Line,
            FmtOp::ForEachEnd,
            FmtOp::GroupEnd,
        ];
        let ops_bytes = encode_ops(ops);
        let fields = &[FieldVal::NodeId(NodeId(99))];
        let format_child = |id: NodeId, arena: &mut DocArena| match id.0 {
            10 => arena.text("aaaa"),
            20 => arena.text("bbbb"),
            _ => panic!("unexpected"),
        };
        let resolve_list = |id: NodeId| match id.0 {
            99 => vec![NodeId(10), NodeId(20)],
            _ => panic!("unexpected"),
        };
        let mut arena = DocArena::new();
        let interp = Interpreter::new(
            dialect, &ops_bytes, ops.len(),
            &format_child, &resolve_list, None, None,
        );
        let doc = interp.run(fields, None, &mut arena);
        assert_eq!(render(&arena, doc, &FormatConfig::default()), "aaaa, bbbb");
        let narrow = FormatConfig { line_width: 5, ..Default::default() };
        assert_eq!(render(&arena, doc, &narrow), "aaaa,\nbbbb");
    }

    #[test]
    fn foreach_empty_list() {
        let td = TestDialect::new(&["a", ", ", "b"], &[]);
        let dialect = td.dialect();
        let ops = &[
            FmtOp::Keyword(0),
            FmtOp::ForEachStart(0),
            FmtOp::ChildItem,
            FmtOp::ForEachSep(1),
            FmtOp::ForEachEnd,
            FmtOp::Keyword(2),
        ];
        let ops_bytes = encode_ops(ops);
        let fields = &[FieldVal::NodeId(NodeId(99))];
        let resolve_list = |id: NodeId| match id.0 {
            99 => vec![],
            _ => panic!("unexpected"),
        };
        let mut arena = DocArena::new();
        let interp = Interpreter::new(
            dialect, &ops_bytes, ops.len(),
            &noop_child, &resolve_list, None, None,
        );
        let doc = interp.run(fields, None, &mut arena);
        assert_eq!(render(&arena, doc, &FormatConfig::default()), "ab");
    }

    #[test]
    fn ifbool_true() {
        let fields = &[FieldVal::Bool(true)];
        let ops = &[
            FmtOp::IfBool(0, 2),
            FmtOp::Keyword(0),
            FmtOp::Else(1),
            FmtOp::Keyword(1),
            FmtOp::EndIf,
        ];
        assert_eq!(run_default(ops, &["YES", "NO"], fields), "YES");
    }

    #[test]
    fn ifbool_false() {
        let fields = &[FieldVal::Bool(false)];
        let ops = &[
            FmtOp::IfBool(0, 2),
            FmtOp::Keyword(0),
            FmtOp::Else(1),
            FmtOp::Keyword(1),
            FmtOp::EndIf,
        ];
        assert_eq!(run_default(ops, &["YES", "NO"], fields), "NO");
    }

    #[test]
    fn ifflag_set() {
        let fields = &[FieldVal::Flags(0b0000_0001)];
        let ops = &[
            FmtOp::IfFlag(0, 1, 2),
            FmtOp::Keyword(0),
            FmtOp::Else(1),
            FmtOp::Keyword(1),
            FmtOp::EndIf,
        ];
        assert_eq!(run_default(ops, &["DISTINCT", "ALL"], fields), "DISTINCT");
    }

    #[test]
    fn ifflag_clear() {
        let fields = &[FieldVal::Flags(0b0000_0000)];
        let ops = &[
            FmtOp::IfFlag(0, 1, 2),
            FmtOp::Keyword(0),
            FmtOp::Else(1),
            FmtOp::Keyword(1),
            FmtOp::EndIf,
        ];
        assert_eq!(run_default(ops, &["DISTINCT", "ALL"], fields), "ALL");
    }

    #[test]
    fn ifenum_match() {
        let fields = &[FieldVal::Enum(1)];
        let ops = &[
            FmtOp::IfEnum(0, 1, 2),
            FmtOp::Keyword(0),
            FmtOp::Else(1),
            FmtOp::Keyword(1),
            FmtOp::EndIf,
        ];
        assert_eq!(run_default(ops, &[" DESC", ""], fields), " DESC");
    }

    #[test]
    fn ifenum_no_match() {
        let fields = &[FieldVal::Enum(0)];
        let ops = &[
            FmtOp::IfEnum(0, 1, 2),
            FmtOp::Keyword(0),
            FmtOp::Else(1),
            FmtOp::Keyword(1),
            FmtOp::EndIf,
        ];
        assert_eq!(run_default(ops, &[" DESC", ""], fields), "");
    }

    #[test]
    fn ifspan_set() {
        let fields = &[FieldVal::Span("hello", 0)];
        let ops = &[
            FmtOp::IfSpan(0, 1),
            FmtOp::Keyword(0),
            FmtOp::EndIf,
        ];
        assert_eq!(run_default(ops, &["HAS_SPAN"], fields), "HAS_SPAN");
    }

    #[test]
    fn ifspan_empty() {
        let fields = &[FieldVal::Span("", 0)];
        let ops = &[
            FmtOp::IfSpan(0, 1),
            FmtOp::Keyword(0),
            FmtOp::EndIf,
        ];
        assert_eq!(run_default(ops, &["HAS_SPAN"], fields), "");
    }

    #[test]
    fn enum_display_maps_ordinal() {
        let fields = &[FieldVal::Enum(2)];
        let enum_display: &[u16] = &[0, 1, 2];
        let ops = &[FmtOp::EnumDisplay(0, 0)];
        assert_eq!(run(ops, &["+", "-", "*"], enum_display, fields, &FormatConfig::default()), "*");
    }

    #[test]
    fn enum_display_with_nonzero_base() {
        let enum_display: &[u16] = &[10, 11, 0, 1];
        let fields = &[FieldVal::Enum(1)];
        let ops = &[FmtOp::EnumDisplay(0, 2)];
        assert_eq!(run(ops, &["AND", "OR"], enum_display, fields, &FormatConfig::default()), "OR");
    }

    #[test]
    fn foreach_self_start() {
        let td = TestDialect::new(&[", "], &[]);
        let dialect = td.dialect();
        let ops = &[
            FmtOp::ForEachSelfStart,
            FmtOp::ChildItem,
            FmtOp::ForEachSep(0),
            FmtOp::ForEachEnd,
        ];
        let ops_bytes = encode_ops(ops);
        let children = &[NodeId(10), NodeId(20), NodeId(30)];
        let format_child = |id: NodeId, arena: &mut DocArena| match id.0 {
            10 => arena.text("x"),
            20 => arena.text("y"),
            30 => arena.text("z"),
            _ => panic!("unexpected"),
        };
        let mut arena = DocArena::new();
        let interp = Interpreter::new(
            dialect, &ops_bytes, ops.len(),
            &format_child, &no_lists, None, None,
        );
        let doc = interp.run(&[], Some(children), &mut arena);
        assert_eq!(render(&arena, doc, &FormatConfig::default()), "x, y, z");
    }

    #[test]
    fn foreach_self_empty() {
        let td = TestDialect::new(&["[", ", ", "]"], &[]);
        let dialect = td.dialect();
        let ops = &[
            FmtOp::Keyword(0),
            FmtOp::ForEachSelfStart,
            FmtOp::ChildItem,
            FmtOp::ForEachSep(1),
            FmtOp::ForEachEnd,
            FmtOp::Keyword(2),
        ];
        let ops_bytes = encode_ops(ops);
        let children: &[NodeId] = &[];
        let mut arena = DocArena::new();
        let interp = Interpreter::new(
            dialect, &ops_bytes, ops.len(),
            &noop_child, &no_lists, None, None,
        );
        let doc = interp.run(&[], Some(children), &mut arena);
        assert_eq!(render(&arena, doc, &FormatConfig::default()), "[]");
    }
}
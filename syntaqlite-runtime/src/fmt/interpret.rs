use crate::parser::{FieldVal, NodeId, Session};

use super::doc::{DocArena, DocId};
use super::format::{first_source_offset, NodeInfo};
use super::ops::FmtOp;
use super::trivia::{flush_trivia, TriviaCtx};

/// Session-level context shared across all interpret calls during formatting.
pub struct FmtCtx<'a> {
    pub strings: &'a [String],
    pub enum_display: &'a [u16],
}

/// Trivia context passed into the interpreter for comment interleaving.
pub struct InterpretTrivia<'a> {
    pub ctx: &'a TriviaCtx<'a>,
    pub session: &'a Session<'a>,
    pub node_info: &'a NodeInfo,
}

// -- Stack frames for the interpreter ----------------------------------------

enum StackFrame {
    Group(Vec<DocId>),
    Nest(i16, Vec<DocId>),
}

struct ForEachState {
    children: Vec<NodeId>,
    index: usize,
    body_start: usize,
}

/// Interpret an FmtOp bytecode array into a Doc tree.
///
/// `fields` contains typed values extracted from the node struct by generated code.
/// `list_children` is set when the current node is a list (for `ForEachSelfStart`).
/// `format_child` recursively formats a child node by ID.
/// `resolve_list` returns the child IDs of a list node.
/// `trivia` optionally provides comment interleaving context.
pub fn interpret<'a>(
    ops: &[FmtOp],
    ctx: &FmtCtx<'a>,
    fields: &[FieldVal<'a>],
    list_children: Option<&[NodeId]>,
    arena: &mut DocArena<'a>,
    format_child: &dyn Fn(NodeId, &mut DocArena<'a>) -> DocId,
    resolve_list: &dyn Fn(NodeId) -> Vec<NodeId>,
    trivia: Option<&InterpretTrivia<'a>>,
) -> DocId {
    let mut parts: Vec<DocId> = Vec::new();
    let mut stack: Vec<StackFrame> = Vec::new();
    let mut for_each_stack: Vec<ForEachState> = Vec::new();
    let mut pending_lines: Vec<DocId> = Vec::new();
    let mut ip: usize = 0;

    while ip < ops.len() {
        match ops[ip] {
            FmtOp::Keyword(sid) => {
                if trivia.is_some() {
                    parts.extend(pending_lines.drain(..));
                }
                parts.push(arena.keyword(&ctx.strings[sid as usize]));
            }
            FmtOp::Span(idx) => {
                let FieldVal::Span(s, offset) = fields[idx as usize] else {
                    panic!("Span: field {} is not a Span", idx);
                };
                if !s.is_empty() {
                    if let Some(it) = trivia {
                        let drain = it.ctx.drain_before(offset, arena);
                        flush_trivia(drain, &mut pending_lines, &mut parts);
                        it.ctx.set_source_end(offset + s.len() as u32);
                    }
                    parts.push(arena.text(s));
                }
            }
            FmtOp::Child(idx) => {
                let FieldVal::NodeId(child_id) = fields[idx as usize] else {
                    panic!("Child: field {} is not a NodeId", idx);
                };
                if !child_id.is_null() {
                    if let Some(it) = trivia {
                        if let Some(offset) = first_source_offset(it.session, it.node_info, child_id) {
                            let drain = it.ctx.drain_before(offset, arena);
                            flush_trivia(drain, &mut pending_lines, &mut parts);
                        } else {
                            parts.extend(pending_lines.drain(..));
                        }
                    }
                    parts.push(format_child(child_id, arena));
                }
            }
            FmtOp::Line => {
                if trivia.is_some() {
                    pending_lines.push(arena.line());
                } else {
                    parts.push(arena.line());
                }
            }
            FmtOp::SoftLine => {
                if trivia.is_some() {
                    pending_lines.push(arena.softline());
                } else {
                    parts.push(arena.softline());
                }
            }
            FmtOp::HardLine => {
                if trivia.is_some() {
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
                } else if let Some(it) = trivia {
                    // Drain trivia before this clause's source range.
                    if let Some(offset) = first_source_offset(it.session, it.node_info, id) {
                        let drain = it.ctx.drain_before(offset, arena);
                        flush_trivia(drain, &mut pending_lines, &mut parts);
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
                    ip = skip_to_foreach_end(ops, ip);
                } else {
                    let children = resolve_list(list_id);
                    if children.is_empty() {
                        ip = skip_to_foreach_end(ops, ip);
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
                if let Some(it) = trivia {
                    if let Some(offset) = first_source_offset(it.session, it.node_info, child_id) {
                        let drain = it.ctx.drain_before(offset, arena);
                        flush_trivia(drain, &mut pending_lines, &mut parts);
                    } else {
                        parts.extend(pending_lines.drain(..));
                    }
                }
                parts.push(format_child(child_id, arena));
            }
            FmtOp::ForEachSep(sid) => {
                let state = for_each_stack.last().expect("ForEachSep outside ForEach");
                if state.index < state.children.len() - 1 {
                    parts.push(arena.text(&ctx.strings[sid as usize]));
                } else {
                    ip = skip_to_foreach_end(ops, ip);
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
                if trivia.is_some() {
                    parts.extend(pending_lines.drain(..));
                }
                let string_id = ctx.enum_display[base as usize + ordinal as usize];
                parts.push(arena.keyword(&ctx.strings[string_id as usize]));
            }
            FmtOp::ForEachSelfStart => {
                let children = list_children.expect("ForEachSelfStart on non-list node");
                if children.is_empty() {
                    ip = skip_to_foreach_end(ops, ip);
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

/// Find the matching ForEachEnd scanning forward from `from_ip`.
fn skip_to_foreach_end(ops: &[FmtOp], from_ip: usize) -> usize {
    let mut depth = 1;
    let mut ip = from_ip + 1;
    while ip < ops.len() {
        match ops[ip] {
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

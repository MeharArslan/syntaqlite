use crate::doc::{DocArena, DocId};
use crate::ops::FmtOp;

const NULL_NODE: u32 = 0xFFFF_FFFF;

/// A typed field value extracted from a node struct by generated code.
#[derive(Clone, Copy, Debug)]
pub enum FieldVal<'a> {
    /// u32 node ID (for `index` fields — child nodes and list references).
    NodeId(u32),
    /// Source text from a `SyntaqliteSourceSpan` field.
    Span(&'a str),
    /// Boolean value (from `Bool` enum, repr(u32)).
    Bool(bool),
    /// Flags byte (from a flags union, repr(transparent) u8).
    Flags(u8),
    /// Enum ordinal (from a value enum, repr(u32)).
    Enum(u32),
}

/// Session-level context shared across all interpret calls during formatting.
pub struct FmtCtx<'a> {
    pub strings: &'a [&'a str],
    pub enum_display: &'a [u16],
}

// -- Stack frames for the interpreter ----------------------------------------

enum StackFrame {
    Group(Vec<DocId>),
    Nest(i16, Vec<DocId>),
}

struct ForEachState {
    children: Vec<u32>,
    index: usize,
    body_start: usize,
}

/// Interpret an FmtOp bytecode array into a Doc tree.
///
/// `fields` contains typed values extracted from the node struct by generated code.
/// `list_children` is set when the current node is a list (for `ForEachSelfStart`).
/// `format_child` recursively formats a child node by ID.
/// `resolve_list` returns the child IDs of a list node.
pub fn interpret<'a>(
    ops: &[FmtOp],
    ctx: &FmtCtx<'a>,
    fields: &[FieldVal<'a>],
    list_children: Option<&[u32]>,
    arena: &mut DocArena<'a>,
    format_child: &dyn Fn(u32, &mut DocArena<'a>) -> DocId,
    resolve_list: &dyn Fn(u32) -> Vec<u32>,
) -> DocId {
    let mut parts: Vec<DocId> = Vec::new();
    let mut stack: Vec<StackFrame> = Vec::new();
    let mut for_each_stack: Vec<ForEachState> = Vec::new();
    let mut ip: usize = 0;

    while ip < ops.len() {
        match ops[ip] {
            FmtOp::Keyword(sid) => {
                parts.push(arena.keyword(ctx.strings[sid as usize]));
            }
            FmtOp::Span(idx) => {
                let FieldVal::Span(s) = fields[idx as usize] else {
                    panic!("Span: field {} is not a Span", idx);
                };
                if !s.is_empty() {
                    parts.push(arena.text(s));
                }
            }
            FmtOp::Child(idx) => {
                let FieldVal::NodeId(child_id) = fields[idx as usize] else {
                    panic!("Child: field {} is not a NodeId", idx);
                };
                if child_id != NULL_NODE {
                    parts.push(format_child(child_id, arena));
                }
            }
            FmtOp::Line => parts.push(arena.line()),
            FmtOp::SoftLine => parts.push(arena.softline()),
            FmtOp::HardLine => parts.push(arena.hardline()),
            FmtOp::GroupStart => {
                stack.push(StackFrame::Group(std::mem::take(&mut parts)));
            }
            FmtOp::GroupEnd => {
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
                if id == NULL_NODE {
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
                if list_id == NULL_NODE {
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
                parts.push(format_child(child_id, arena));
            }
            FmtOp::ForEachSep(sid) => {
                let state = for_each_stack.last().expect("ForEachSep outside ForEach");
                if state.index < state.children.len() - 1 {
                    parts.push(arena.text(ctx.strings[sid as usize]));
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
                let FieldVal::Span(s) = fields[idx as usize] else {
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
                let string_id = ctx.enum_display[base as usize + ordinal as usize];
                parts.push(arena.keyword(ctx.strings[string_id as usize]));
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

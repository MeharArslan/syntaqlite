use std::fmt::Write;

use crate::fields::{FieldKind, FieldVal};
use crate::generated::nodes::{FIELD_DESCRIPTORS, NODE_NAMES};
use crate::nodes::NULL_NODE;
use crate::Session;

/// Dump an AST node tree as indented text.
pub fn dump_node(session: &Session<'_>, id: u32, out: &mut String, indent: usize) {
    if id == NULL_NODE {
        return;
    }
    let Some(node) = session.node(id) else {
        return;
    };
    let source = session.source();
    let pad = "  ".repeat(indent);
    let tag = node.tag() as usize;

    if let Some(list) = node.as_list() {
        let _ = writeln!(out, "{pad}{} [{} items]", NODE_NAMES[tag], list.count);
        for &child_id in list.children() {
            dump_node(session, child_id, out, indent + 1);
        }
        return;
    }

    let _ = writeln!(out, "{pad}{}", NODE_NAMES[tag]);
    let descriptors = FIELD_DESCRIPTORS[tag];
    let fields = node.fields(source);

    for (desc, val) in descriptors.iter().zip(fields.iter()) {
        match (val, &desc.kind) {
            (FieldVal::NodeId(child_id), _) => {
                if *child_id == NULL_NODE {
                    let _ = writeln!(out, "{pad}  {}: (none)", desc.name);
                } else {
                    let _ = writeln!(out, "{pad}  {}:", desc.name);
                    dump_node(session, *child_id, out, indent + 2);
                }
            }
            (FieldVal::Span(text, _), _) => {
                if text.is_empty() {
                    let _ = writeln!(out, "{pad}  {}: null", desc.name);
                } else {
                    let _ = writeln!(out, "{pad}  {}: \"{text}\"", desc.name);
                }
            }
            (FieldVal::Bool(b), _) => {
                let s = if *b { "TRUE" } else { "FALSE" };
                let _ = writeln!(out, "{pad}  {}: {s}", desc.name);
            }
            (FieldVal::Flags(v), FieldKind::Flags(display)) => {
                let _ = writeln!(out, "{pad}  {}: {}", desc.name, display(*v));
            }
            (FieldVal::Enum(v), FieldKind::Enum(display)) => {
                let s = display(*v).unwrap_or("?");
                let _ = writeln!(out, "{pad}  {}: {s}", desc.name);
            }
            _ => {}
        }
    }
}

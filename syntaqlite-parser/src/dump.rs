use std::fmt::Write;

use crate::nodes::{SourceSpan, NULL_NODE};
use crate::Session;

// Re-export the generated dispatch function.
pub use crate::generated::dump::dump_node;

/// Print a child node field (index field).
pub(crate) fn dump_child(
    session: &Session<'_>,
    pad: &str,
    name: &str,
    id: u32,
    out: &mut String,
    indent: usize,
) {
    if id == NULL_NODE {
        let _ = writeln!(out, "{pad}  {name}: (none)");
    } else {
        let _ = writeln!(out, "{pad}  {name}:");
        dump_node(session, id, out, indent + 2);
    }
}

/// Print a SourceSpan field (inline span).
pub(crate) fn dump_span(
    pad: &str,
    name: &str,
    span: &SourceSpan,
    source: &str,
    out: &mut String,
) {
    if span.is_empty() {
        let _ = writeln!(out, "{pad}  {name}: null");
    } else {
        let text = span.as_str(source);
        let _ = writeln!(out, "{pad}  {name}: \"{text}\"");
    }
}

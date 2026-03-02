// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! JSON serialization extension for [`NodeRef`].
//!
//! Gated on `feature = "json"`. Import [`NodeRefJsonExt`] to call
//! [`dump_json`](NodeRefJsonExt::dump_json) on a [`NodeRef`].

use syntaqlite_parser::Dialect;
use syntaqlite_parser::{FieldVal, NodeId};
use syntaqlite_parser::{NodeRef, RawNodeReader};

/// Extension trait that adds JSON serialization to [`NodeRef`].
///
/// Enabled with `feature = "json"`. Produces JSON matching the WASM AST JSON format.
pub trait NodeRefJsonExt {
    /// Serialize this node to the WASM AST JSON format, appending to `out`.
    fn dump_json(&self, out: &mut String);
}

impl NodeRefJsonExt for NodeRef<'_> {
    fn dump_json(&self, out: &mut String) {
        let value = dump_json_id(self.id(), self.reader(), self.dialect());
        out.push_str(&serde_json::to_string(&value).expect("AST dump serialization failed"));
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn dump_json_id(id: NodeId, reader: RawNodeReader<'_>, dialect: Dialect<'_>) -> serde_json::Value {
    if id.is_null() {
        return serde_json::Value::Null;
    }
    let Some(tag) = reader.node_tag(id) else {
        return serde_json::Value::Null;
    };

    let name = dialect.node_name(tag);

    if dialect.is_list(tag) {
        let children = reader.list_children(id, &dialect).unwrap_or(&[]);
        let child_values: Vec<serde_json::Value> = children
            .iter()
            .map(|&child_id| {
                if child_id.is_null() || reader.node_tag(child_id).is_none() {
                    serde_json::json!({"type": "node", "name": "null", "fields": []})
                } else {
                    dump_json_id(child_id, reader, dialect)
                }
            })
            .collect();
        return serde_json::json!({
            "type": "list",
            "name": name,
            "count": children.len(),
            "children": child_values,
        });
    }

    let meta = dialect.field_meta(tag);
    let Some((_, fields)) = reader.extract_fields(id, &dialect) else {
        return serde_json::Value::Null;
    };

    let field_values: Vec<serde_json::Value> = meta
        .iter()
        .zip(fields.iter())
        .map(|(m, fv)| {
            // SAFETY: m.name is a valid NUL-terminated C string from codegen.
            let label = unsafe { m.name_str() };
            match fv {
                FieldVal::NodeId(child_id) => {
                    let child = if child_id.is_null() {
                        serde_json::Value::Null
                    } else {
                        dump_json_id(*child_id, reader, dialect)
                    };
                    serde_json::json!({"kind": "node", "label": label, "child": child})
                }
                FieldVal::Span(text, _) => {
                    let value = if text.is_empty() {
                        serde_json::Value::Null
                    } else {
                        serde_json::Value::String(text.to_string())
                    };
                    serde_json::json!({"kind": "span", "label": label, "value": value})
                }
                FieldVal::Bool(val) => {
                    serde_json::json!({"kind": "bool", "label": label, "value": val})
                }
                FieldVal::Enum(val) => {
                    // SAFETY: m.display is a valid C array from codegen.
                    let value = unsafe { m.display_name(*val as usize) }
                        .map(|s| serde_json::Value::String(s.to_string()))
                        .unwrap_or(serde_json::Value::Null);
                    serde_json::json!({"kind": "enum", "label": label, "value": value})
                }
                FieldVal::Flags(val) => {
                    let flag_values: Vec<serde_json::Value> = (0..8u8)
                        .filter(|&bit| val & (1 << bit) != 0)
                        .map(|bit| {
                            // SAFETY: m.display is a valid C array from codegen.
                            match unsafe { m.display_name(bit as usize) } {
                                Some(s) => serde_json::Value::String(s.to_string()),
                                None => serde_json::json!(1u32 << bit),
                            }
                        })
                        .collect();
                    serde_json::json!({"kind": "flags", "label": label, "value": flag_values})
                }
            }
        })
        .collect();

    serde_json::json!({
        "type": "node",
        "name": name,
        "fields": field_values,
    })
}

#[cfg(test)]
#[cfg(feature = "sqlite")]
mod tests {
    use super::*;
    use syntaqlite_parser::RawParser;

    #[test]
    fn node_ref_dump_json_produces_valid_json() {
        let dialect = crate::dialect::sqlite();
        let mut parser = RawParser::builder(dialect).build();
        let mut cursor = parser.parse("SELECT 1;");
        let node = cursor.next_statement().unwrap().unwrap();
        let mut out = String::new();
        node.dump_json(&mut out);
        assert!(
            out.contains("\"type\":\"node\""),
            "expected node type in JSON"
        );
        assert!(out.ends_with('}'));
    }
}

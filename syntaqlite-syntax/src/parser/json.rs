// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! JSON serialization helpers for grammar-agnostic AST nodes.
//!
//! Gated on `feature = "json"`.

use crate::any::{AnyNode, AnyNodeId};
use crate::ast::FieldValue;
use crate::parser::AnyParsedStatement;

pub(crate) fn write_json_node(node: &AnyNode<'_>, out: &mut String) {
    let value = json_value_for_id(node.id, node.stmt_result);
    out.push_str(&serde_json::to_string(&value).expect("AST dump serialization failed"));
}

fn json_value_for_id(id: AnyNodeId, stmt_result: AnyParsedStatement<'_>) -> serde_json::Value {
    if id.is_null() {
        return serde_json::Value::Null;
    }

    let Some((tag, fields)) = stmt_result.extract_fields(id) else {
        return serde_json::Value::Null;
    };

    let grammar = stmt_result.grammar;
    let name = grammar.node_name(tag);

    if grammar.is_list(tag) {
        let children = stmt_result.list_children(id).unwrap_or(&[]);
        let child_values: Vec<serde_json::Value> = children
            .iter()
            .map(|&child_id| {
                if child_id.is_null() || stmt_result.node_ptr(child_id).is_none() {
                    serde_json::json!({"type": "node", "name": "null", "fields": []})
                } else {
                    json_value_for_id(child_id, stmt_result)
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

    let meta: Vec<_> = grammar.field_meta(tag).collect();
    let field_values: Vec<serde_json::Value> = meta
        .iter()
        .zip((0..fields.len()).map(|i| fields[i]))
        .map(|(m, fv)| {
            let label = m.name();
            match fv {
                FieldValue::NodeId(child_id) => {
                    let child = if child_id.is_null() {
                        serde_json::Value::Null
                    } else {
                        json_value_for_id(child_id, stmt_result)
                    };
                    serde_json::json!({"kind": "node", "label": label, "child": child})
                }
                FieldValue::Span(text) => {
                    let value = if text.is_empty() {
                        serde_json::Value::Null
                    } else {
                        serde_json::Value::String(text.to_string())
                    };
                    serde_json::json!({"kind": "span", "label": label, "value": value})
                }
                FieldValue::Bool(val) => {
                    serde_json::json!({"kind": "bool", "label": label, "value": val})
                }
                FieldValue::Enum(val) => {
                    let value = m
                        .display_name(val as usize)
                        .map(|s| serde_json::Value::String(s.to_string()))
                        .unwrap_or(serde_json::Value::Null);
                    serde_json::json!({"kind": "enum", "label": label, "value": value})
                }
                FieldValue::Flags(val) => {
                    let flag_values: Vec<serde_json::Value> = (0..8u8)
                        .filter(|&bit| val & (1 << bit) != 0)
                        .map(|bit| match m.display_name(bit as usize) {
                            Some(s) => serde_json::Value::String(s.to_string()),
                            None => serde_json::json!(1u32 << bit),
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
    use crate::any::AnyParser;
    use crate::ast::GrammarNodeType;
    use crate::sqlite::ast::{Node, Stmt};
    use crate::typed::grammar;

    fn dump_json_value<F>(f: F) -> serde_json::Value
    where
        F: FnOnce(&mut String),
    {
        let mut out = String::new();
        f(&mut out);
        serde_json::from_str(&out).expect("json output should be valid")
    }

    #[test]
    fn dump_node_json_produces_valid_json() {
        let parser = AnyParser::new(grammar().into());
        let mut cursor = parser.parse("SELECT 1;");
        let stmt = cursor
            .next()
            .expect("statement is present")
            .expect("statement parses successfully");
        let root_id = stmt.root_id();
        let node = crate::any::AnyNode::from_result(stmt, root_id).expect("root resolves as node");

        let json = dump_json_value(|out| node.dump_json(out));
        assert_eq!(json["type"], "node");
        assert_eq!(json["name"], "SelectStmt");
    }

    #[test]
    fn typed_node_dump_json_matches_any_node_dump_json() {
        let parser = crate::Parser::new();
        let mut session = parser.parse("SELECT a, b FROM tbl;");
        let stmt = session
            .next()
            .expect("statement is present")
            .expect("statement parses successfully");
        let typed_root = stmt.root().expect("root should be present");
        let select = match typed_root {
            Stmt::SelectStmt(n) => n,
            other => panic!("expected SelectStmt root, got {other:?}"),
        };
        let any_stmt = stmt.erase();
        let any_root = crate::any::AnyNode::from_result(any_stmt, select.node_id().into())
            .expect("typed root id resolves to AnyNode");

        let typed_json = dump_json_value(|out| select.dump_json(out));
        let any_json = dump_json_value(|out| any_root.dump_json(out));
        assert_eq!(typed_json, any_json);
    }

    #[test]
    fn node_and_typed_list_dump_json_match_any_node_dump_json() {
        let parser = crate::Parser::new();
        let mut session = parser.parse("SELECT a, b FROM tbl;");
        let stmt = session
            .next()
            .expect("statement is present")
            .expect("statement parses successfully");
        let any_stmt = stmt.erase();
        let root_id = any_stmt.root_id();
        let root_node = Node::resolve(any_stmt, root_id).expect("root node resolves");

        let any_root =
            crate::any::AnyNode::from_result(any_stmt, root_id).expect("root as AnyNode");
        let node_json = dump_json_value(|out| root_node.dump_json(out));
        let any_root_json = dump_json_value(|out| any_root.dump_json(out));
        assert_eq!(node_json, any_root_json);

        let typed_root = stmt.root().expect("root should be present");
        let select = match typed_root {
            Stmt::SelectStmt(n) => n,
            other => panic!("expected SelectStmt root, got {other:?}"),
        };
        let columns = select
            .columns()
            .expect("select columns list should be present");
        let typed_list_json = dump_json_value(|out| columns.dump_json(out));
        let any_list = crate::any::AnyNode::from_result(any_stmt, columns.node_id().into())
            .expect("typed list id resolves to AnyNode");
        let any_list_json = dump_json_value(|out| any_list.dump_json(out));
        assert_eq!(typed_list_json, any_list_json);
        assert_eq!(typed_list_json["type"], "list");
        assert_eq!(typed_list_json["count"], 2);
    }
}

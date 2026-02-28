// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use syntaqlite::Parser;
use syntaqlite::parser::NodeId;
use syntaqlite::sqlite::low_level::dialect;

#[test]
fn child_node_ids_returns_children_for_select() {
    let dialect = dialect();
    let mut parser = Parser::new();
    let mut cursor = parser.parse("SELECT 1 + 2");
    let stmt_id = cursor.next_statement().unwrap().unwrap();
    let reader = cursor.reader();

    let children = reader.child_node_ids(stmt_id, dialect);
    // SelectStmt has fields like columns, from_clause, etc.
    // "SELECT 1 + 2" should have at least a columns child.
    assert!(!children.is_empty(), "SelectStmt should have child nodes");
}

#[test]
fn child_node_ids_returns_empty_for_null_id() {
    let dialect = dialect();
    let mut parser = Parser::new();
    let mut cursor = parser.parse("SELECT 1");
    let _stmt_id = cursor.next_statement().unwrap().unwrap();
    let reader = cursor.reader();

    let children = reader.child_node_ids(NodeId::NULL, dialect);
    assert!(children.is_empty(), "NULL id should have no children");
}

#[test]
fn child_node_ids_includes_list_children() {
    let dialect = dialect();
    let mut parser = Parser::new();
    // A list node (ResultColumnList) should enumerate its children too.
    let mut cursor = parser.parse("SELECT 1, 2, 3");
    let stmt_id = cursor.next_statement().unwrap().unwrap();
    let reader = cursor.reader();

    // Walk one level: SelectStmt → children should include the columns list.
    let children = reader.child_node_ids(stmt_id, dialect);
    assert!(!children.is_empty());

    // Find a list child and verify it also has children.
    let mut found_list_with_children = false;
    for child_id in &children {
        let list_children = reader.child_node_ids(*child_id, dialect);
        if !list_children.is_empty() {
            found_list_with_children = true;
            break;
        }
    }
    assert!(
        found_list_with_children,
        "should find at least one child with its own children"
    );
}

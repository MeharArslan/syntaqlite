// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use syntaqlite::ast::{Node, NodeTag};
use syntaqlite::dialect::sqlite as dialect;
use syntaqlite::ext::{DialectNodeType, NodeId, RawParser, RawStatementCursor};

fn new_parser() -> RawParser<'static> {
    RawParser::builder(dialect()).build()
}

/// Helper: resolve a NodeId to its Node variant and return its tag.
fn node_tag(cursor: &RawStatementCursor, id: NodeId) -> NodeTag {
    let node: Node =
        DialectNodeType::from_arena(cursor.reader(), id).expect("should resolve to a Node");
    node.tag()
}

#[test]
fn select_children_include_result_column_list_and_table_ref() {
    let dialect = dialect();
    let mut parser = new_parser();
    let mut cursor = parser.parse("SELECT a, b FROM t");
    let stmt_id = cursor.next_statement().unwrap().unwrap().id();

    let children = cursor.reader().child_node_ids(stmt_id, &dialect);
    // SelectStmt("SELECT a, b FROM t") should have exactly 2 non-null children:
    // a ResultColumnList and a TableRef.
    assert_eq!(
        children.len(),
        2,
        "SelectStmt should have 2 children (columns + from)"
    );

    let tags: Vec<_> = children.iter().map(|id| node_tag(&cursor, *id)).collect();
    assert!(
        tags.contains(&NodeTag::ResultColumnList),
        "children should include ResultColumnList, got: {tags:?}"
    );
    assert!(
        tags.contains(&NodeTag::TableRef),
        "children should include TableRef, got: {tags:?}"
    );
}

#[test]
fn null_id_returns_empty() {
    let dialect = dialect();
    let mut parser = new_parser();
    let mut cursor = parser.parse("SELECT 1");
    let _stmt_id = cursor.next_statement().unwrap().unwrap().id();

    assert!(
        cursor
            .reader()
            .child_node_ids(NodeId::NULL, &dialect)
            .is_empty()
    );
}

#[test]
fn list_node_enumerates_its_elements() {
    let dialect = dialect();
    let mut parser = new_parser();
    let mut cursor = parser.parse("SELECT a, b, c");
    let stmt_id = cursor.next_statement().unwrap().unwrap().id();

    // Find the ResultColumnList child of SelectStmt.
    let children = cursor.reader().child_node_ids(stmt_id, &dialect);
    let list_id = children
        .iter()
        .find(|id| node_tag(&cursor, **id) == NodeTag::ResultColumnList)
        .expect("SelectStmt should have a ResultColumnList child");

    // The list should contain exactly 3 ResultColumn children.
    let list_children = cursor.reader().child_node_ids(*list_id, &dialect);
    assert_eq!(
        list_children.len(),
        3,
        "ResultColumnList for 'a, b, c' should have 3 children"
    );
    for (i, child_id) in list_children.iter().enumerate() {
        assert_eq!(
            node_tag(&cursor, *child_id),
            NodeTag::ResultColumn,
            "list child {i} should be a ResultColumn"
        );
    }
}

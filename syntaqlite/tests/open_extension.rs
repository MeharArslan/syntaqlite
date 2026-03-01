// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use syntaqlite::ast::{Node, Stmt};
use syntaqlite::raw::{DialectNodeType, RawParser};

#[test]
fn pure_sqlite_never_produces_node_other() {
    let mut parser = RawParser::new();
    let sqls = [
        "SELECT 1 + 2",
        "INSERT INTO t VALUES(1)",
        "UPDATE t SET x = 1",
        "DELETE FROM t WHERE x = 1",
        "CREATE TABLE t(x INT)",
    ];
    for sql in &sqls {
        let mut cursor = parser.parse(sql);
        let node_ref = cursor.next_statement().unwrap().unwrap();
        let reader = cursor.reader();
        let node: Option<Node> = DialectNodeType::from_arena(reader, node_ref.id());
        assert!(node.is_some(), "should resolve: {sql}");
        match node.unwrap() {
            Node::Other { .. } => panic!("unexpected Node::Other for pure SQLite: {sql}"),
            _ => {}
        }
    }
}

#[test]
fn pure_sqlite_never_produces_stmt_other() {
    let mut parser = RawParser::new();
    let sqls = [
        "SELECT 1",
        "INSERT INTO t VALUES(1)",
        "UPDATE t SET x = 1",
        "DELETE FROM t WHERE x = 1",
    ];
    for sql in &sqls {
        let mut cursor = parser.parse(sql);
        let node_ref = cursor.next_statement().unwrap().unwrap();
        let reader = cursor.reader();
        let stmt: Option<Stmt> = DialectNodeType::from_arena(reader, node_ref.id());
        assert!(stmt.is_some(), "should resolve as Stmt: {sql}");
        match stmt.unwrap() {
            Stmt::Other(_) => panic!("unexpected Stmt::Other for pure SQLite: {sql}"),
            _ => {}
        }
    }
}

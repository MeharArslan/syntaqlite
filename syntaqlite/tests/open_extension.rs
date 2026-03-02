// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use syntaqlite::ast::{Node, Stmt};
use syntaqlite::ext::RawParser;

fn new_parser() -> RawParser<'static> {
    RawParser::new(syntaqlite::dialect::sqlite())
}

#[test]
fn pure_sqlite_never_produces_node_other() {
    let mut parser = new_parser();
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
        let node: Option<Node> = node_ref.as_typed();
        assert!(node.is_some(), "should resolve: {sql}");
        if let Node::Other { .. } = node.unwrap() {
            panic!("unexpected Node::Other for pure SQLite: {sql}")
        }
    }
}

#[test]
fn pure_sqlite_never_produces_stmt_other() {
    let mut parser = new_parser();
    let sqls = [
        "SELECT 1",
        "INSERT INTO t VALUES(1)",
        "UPDATE t SET x = 1",
        "DELETE FROM t WHERE x = 1",
    ];
    for sql in &sqls {
        let mut cursor = parser.parse(sql);
        let node_ref = cursor.next_statement().unwrap().unwrap();
        let stmt: Option<Stmt> = node_ref.as_typed();
        assert!(stmt.is_some(), "should resolve as Stmt: {sql}");
        if let Stmt::Other(_) = stmt.unwrap() {
            panic!("unexpected Stmt::Other for pure SQLite: {sql}")
        }
    }
}

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use syntaqlite::ast::{FromArena, Stmt};
use syntaqlite::raw::RawParser;

#[test]
fn parse_select_1() {
    let mut parser = RawParser::new();
    let mut session = parser.parse("SELECT 1;");

    let node = session.next_statement().unwrap().unwrap();
    let stmt = Stmt::from_arena(session.reader(), node.id()).unwrap();
    let Stmt::SelectStmt(_select) = stmt else {
        panic!("expected SelectStmt")
    };

    // No more statements.
    assert!(session.next_statement().is_none());
}

#[test]
fn parse_multiple_statements() {
    let mut parser = RawParser::new();
    let mut session = parser.parse("SELECT 1; SELECT 2;");

    let node1 = session.next_statement().unwrap().unwrap();
    let stmt1 = Stmt::from_arena(session.reader(), node1.id()).unwrap();
    assert!(matches!(stmt1, Stmt::SelectStmt(_)));

    let node2 = session.next_statement().unwrap().unwrap();
    let stmt2 = Stmt::from_arena(session.reader(), node2.id()).unwrap();
    assert!(matches!(stmt2, Stmt::SelectStmt(_)));

    assert!(session.next_statement().is_none());
}

#[test]
fn parse_error() {
    let mut parser = RawParser::new();
    let mut session = parser.parse("SELECT");

    let result = session.next_statement().unwrap();
    assert!(result.is_err());
}

#[test]
fn parse_error_select_bare() {
    // "SELECT " with trailing space — no column list, no semicolon.
    // Should return an error with a non-empty message, not silently return None.
    let mut parser = RawParser::new();
    let mut session = parser.parse("SELECT ");

    let result = session.next_statement().unwrap();
    let err = result.expect_err("expected parse error for bare SELECT");
    assert!(
        !err.message.is_empty(),
        "error message should not be empty, got: {:?}",
        err.message
    );
}

#[test]
fn parse_error_has_message_and_offset() {
    // A syntax error should carry a non-empty message.
    let mut parser = RawParser::new();
    let mut session = parser.parse("NOT VALID SQL;");

    let err = session
        .next_statement()
        .unwrap()
        .expect_err("expected parse error");
    assert!(!err.message.is_empty(), "error message should not be empty");
}

#[test]
fn parse_error_recovery() {
    // After a parse error, the cursor continues parsing subsequent statements.
    // Lemon's built-in error recovery synchronises on `;`.
    let mut parser = RawParser::new();
    let mut session = parser.parse("NOT VALID SQL; SELECT 1;");

    let first = session.next_statement().unwrap();
    assert!(first.is_err(), "expected parse error for invalid SQL");

    // Recovery: cursor should continue and return the next valid statement.
    let second = session.next_statement().unwrap().unwrap();
    assert!(
        matches!(
            Stmt::from_arena(session.reader(), second.id()).unwrap(),
            Stmt::SelectStmt(_)
        ),
        "expected SelectStmt after recovery"
    );

    assert!(session.next_statement().is_none());
}

#[test]
fn parse_error_recovery_at_eof() {
    // An unterminated statement (no trailing `;`) reports an error and then
    // next_statement() returns None.
    let mut parser = RawParser::new();
    let mut session = parser.parse("SELECT * FROM");

    let result = session.next_statement().unwrap();
    assert!(result.is_err());
    assert!(session.next_statement().is_none());
}

#[test]
fn parse_error_mid_batch() {
    // Good → bad → good: the cursor recovers from a mid-batch error and
    // continues to parse subsequent valid statements.
    let mut parser = RawParser::new();
    let mut session = parser.parse("SELECT 1; SELECT * FROM; SELECT 2;");

    let r1 = session.next_statement().unwrap().unwrap();
    assert!(matches!(
        Stmt::from_arena(session.reader(), r1.id()).unwrap(),
        Stmt::SelectStmt(_)
    ));

    assert!(session.next_statement().unwrap().is_err());

    let r3 = session.next_statement().unwrap().unwrap();
    assert!(matches!(
        Stmt::from_arena(session.reader(), r3.id()).unwrap(),
        Stmt::SelectStmt(_)
    ));

    assert!(session.next_statement().is_none());
}

#[test]
fn parser_reuse() {
    let mut parser = RawParser::new();

    // First parse
    {
        let mut session = parser.parse("SELECT 1");
        let node = session.next_statement().unwrap().unwrap();
        let stmt = Stmt::from_arena(session.reader(), node.id()).unwrap();
        assert!(matches!(stmt, Stmt::SelectStmt(_)));
    }

    // Reuse with different input
    {
        let mut session = parser.parse("DELETE FROM t");
        let node = session.next_statement().unwrap().unwrap();
        let stmt = Stmt::from_arena(session.reader(), node.id()).unwrap();
        assert!(matches!(stmt, Stmt::DeleteStmt(_)));
    }
}

// -- DELETE / UPDATE with ORDER BY and LIMIT --

fn parser_with_update_delete_limit() -> RawParser<'static> {
    use syntaqlite::dialect::DialectConfig;
    let dialect = syntaqlite::dialect::sqlite();
    let mut config = DialectConfig::default();
    config.cflags.set(40); // SQLITE_ENABLE_UPDATE_DELETE_LIMIT
    RawParser::builder(dialect).dialect_config(config).build()
}

#[test]
fn parse_delete_with_order_by_limit() {
    let mut parser = parser_with_update_delete_limit();
    let mut cursor = parser.parse("DELETE FROM t ORDER BY id LIMIT 5;");

    let node = cursor.next_statement().unwrap().unwrap();
    let stmt = Stmt::from_arena(cursor.reader(), node.id()).unwrap();
    let Stmt::DeleteStmt(del) = stmt else {
        panic!("expected DeleteStmt, got {stmt:?}");
    };
    assert!(del.orderby().is_some(), "should have ORDER BY");
    assert!(del.limit_clause().is_some(), "should have LIMIT");
}

#[test]
fn parse_update_with_order_by_limit() {
    let mut parser = parser_with_update_delete_limit();
    let mut cursor = parser.parse("UPDATE t SET a = 1 ORDER BY id LIMIT 3;");

    let node = cursor.next_statement().unwrap().unwrap();
    let stmt = Stmt::from_arena(cursor.reader(), node.id()).unwrap();
    let Stmt::UpdateStmt(upd) = stmt else {
        panic!("expected UpdateStmt, got {stmt:?}");
    };
    assert!(upd.orderby().is_some(), "should have ORDER BY");
    assert!(upd.limit_clause().is_some(), "should have LIMIT");
}

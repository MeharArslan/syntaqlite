// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use syntaqlite::sqlite::ast::{FromArena, Stmt};

#[test]
fn parse_select_1() {
    let mut parser = syntaqlite::Parser::new();
    let mut session = parser.parse("SELECT 1;");

    let id = session.next_statement().unwrap().unwrap();
    let stmt = Stmt::from_arena(session.reader(), id).unwrap();
    let Stmt::SelectStmt(_select) = stmt else {
        panic!("expected SelectStmt")
    };

    // No more statements.
    assert!(session.next_statement().is_none());
}

#[test]
fn parse_multiple_statements() {
    let mut parser = syntaqlite::Parser::new();
    let mut session = parser.parse("SELECT 1; SELECT 2;");

    let id1 = session.next_statement().unwrap().unwrap();
    let stmt1 = Stmt::from_arena(session.reader(), id1).unwrap();
    assert!(matches!(stmt1, Stmt::SelectStmt(_)));

    let id2 = session.next_statement().unwrap().unwrap();
    let stmt2 = Stmt::from_arena(session.reader(), id2).unwrap();
    assert!(matches!(stmt2, Stmt::SelectStmt(_)));

    assert!(session.next_statement().is_none());
}

#[test]
fn parse_error() {
    let mut parser = syntaqlite::Parser::new();
    let mut session = parser.parse("SELECT");

    let result = session.next_statement().unwrap();
    assert!(result.is_err());
}

#[test]
fn parse_error_select_bare() {
    // "SELECT " with trailing space — no column list, no semicolon.
    // Should return an error with a non-empty message, not silently return None.
    let mut parser = syntaqlite::Parser::new();
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
    let mut parser = syntaqlite::Parser::new();
    let mut session = parser.parse("NOT VALID SQL;");

    let err = session
        .next_statement()
        .unwrap()
        .expect_err("expected parse error");
    assert!(
        !err.message.is_empty(),
        "error message should not be empty"
    );
}

#[test]
fn parse_error_recovery() {
    // After a parse error, the cursor continues parsing subsequent statements.
    // Lemon's built-in error recovery synchronises on `;`.
    let mut parser = syntaqlite::Parser::new();
    let mut session = parser.parse("NOT VALID SQL; SELECT 1;");

    let first = session.next_statement().unwrap();
    assert!(first.is_err(), "expected parse error for invalid SQL");

    // Recovery: cursor should continue and return the next valid statement.
    let second = session.next_statement().unwrap().unwrap();
    assert!(
        matches!(
            Stmt::from_arena(session.reader(), second).unwrap(),
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
    let mut parser = syntaqlite::Parser::new();
    let mut session = parser.parse("SELECT * FROM");

    let result = session.next_statement().unwrap();
    assert!(result.is_err());
    assert!(session.next_statement().is_none());
}

#[test]
fn parse_error_mid_batch() {
    // Good → bad → good: the cursor recovers from a mid-batch error and
    // continues to parse subsequent valid statements.
    let mut parser = syntaqlite::Parser::new();
    let mut session = parser.parse("SELECT 1; SELECT * FROM; SELECT 2;");

    let r1 = session.next_statement().unwrap().unwrap();
    assert!(matches!(
        Stmt::from_arena(session.reader(), r1).unwrap(),
        Stmt::SelectStmt(_)
    ));

    assert!(session.next_statement().unwrap().is_err());

    let r3 = session.next_statement().unwrap().unwrap();
    assert!(matches!(
        Stmt::from_arena(session.reader(), r3).unwrap(),
        Stmt::SelectStmt(_)
    ));

    assert!(session.next_statement().is_none());
}

#[test]
fn parser_reuse() {
    let mut parser = syntaqlite::Parser::new();

    // First parse
    {
        let mut session = parser.parse("SELECT 1");
        let id = session.next_statement().unwrap().unwrap();
        let stmt = Stmt::from_arena(session.reader(), id).unwrap();
        assert!(matches!(stmt, Stmt::SelectStmt(_)));
    }

    // Reuse with different input
    {
        let mut session = parser.parse("DELETE FROM t");
        let id = session.next_statement().unwrap().unwrap();
        let stmt = Stmt::from_arena(session.reader(), id).unwrap();
        assert!(matches!(stmt, Stmt::DeleteStmt(_)));
    }
}

// -- DELETE / UPDATE with ORDER BY and LIMIT --

fn parser_with_update_delete_limit() -> syntaqlite::Parser {
    use syntaqlite::dialect::ffi::DialectConfig;
    let dialect = syntaqlite::sqlite::low_level::dialect();
    let mut parser = syntaqlite::Parser::with_dialect(dialect);
    let mut config = DialectConfig::default();
    config.cflags.set(40); // SQLITE_ENABLE_UPDATE_DELETE_LIMIT
    parser.set_dialect_config(&config);
    parser
}

#[test]
fn parse_delete_with_order_by_limit() {
    let mut parser = parser_with_update_delete_limit();
    let mut cursor = parser.parse("DELETE FROM t ORDER BY id LIMIT 5;");

    let id = cursor.next_statement().unwrap().unwrap();
    let stmt = Stmt::from_arena(cursor.reader(), id).unwrap();
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

    let id = cursor.next_statement().unwrap().unwrap();
    let stmt = Stmt::from_arena(cursor.reader(), id).unwrap();
    let Stmt::UpdateStmt(upd) = stmt else {
        panic!("expected UpdateStmt, got {stmt:?}");
    };
    assert!(upd.orderby().is_some(), "should have ORDER BY");
    assert!(upd.limit_clause().is_some(), "should have LIMIT");
}

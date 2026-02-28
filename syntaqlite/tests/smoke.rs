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
fn parse_error_poisons_cursor() {
    // StatementCursor is intentionally poisoned on error: once an error is
    // returned, all subsequent next_statement() calls return None. Callers
    // that need error recovery must create a new session.
    let mut parser = syntaqlite::Parser::new();
    let mut session = parser.parse("NOT VALID SQL; SELECT 1;");

    let first = session.next_statement().unwrap();
    assert!(first.is_err(), "expected parse error for invalid SQL");

    // Cursor is now poisoned — further calls return None, not a result.
    assert!(
        session.next_statement().is_none(),
        "poisoned cursor should return None after an error"
    );
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

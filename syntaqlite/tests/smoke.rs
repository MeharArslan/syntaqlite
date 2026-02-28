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

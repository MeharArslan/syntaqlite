// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use syntaqlite::ast::Stmt;


#[test]
fn parse_select_1() {
    let mut parser = syntaqlite::Parser::new();
    let mut session = parser.parse("SELECT 1;");

    let stmt = session.next_statement().unwrap().unwrap();
    let Stmt::SelectStmt(_select) = stmt else { panic!("expected SelectStmt") };

    // No more statements.
    assert!(session.next_statement().is_none());
}

#[test]
fn parse_multiple_statements() {
    let mut parser = syntaqlite::Parser::new();
    let mut session = parser.parse("SELECT 1; SELECT 2;");

    let stmt1 = session.next_statement().unwrap().unwrap();
    assert!(matches!(stmt1, Stmt::SelectStmt(_)));

    let stmt2 = session.next_statement().unwrap().unwrap();
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
fn parser_reuse() {
    let mut parser = syntaqlite::Parser::new();

    // First parse
    {
        let mut session = parser.parse("SELECT 1");
        let stmt = session.next_statement().unwrap().unwrap();
        assert!(matches!(stmt, Stmt::SelectStmt(_)));
    }

    // Reuse with different input
    {
        let mut session = parser.parse("DELETE FROM t");
        let stmt = session.next_statement().unwrap().unwrap();
        assert!(matches!(stmt, Stmt::DeleteStmt(_)));
    }
}

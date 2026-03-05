// TODO: broken - needs migration to syntaqlite_syntax
#![cfg(broken_needs_migration)]

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use syntaqlite::TokenType;
use syntaqlite::ast::Stmt;
use syntaqlite::incremental::IncrementalParser;

/// Feed tokens for "SELECT 1" via the low-level API and verify same AST
/// as the high-level parse.
#[test]
fn feed_tokens_select_1() {
    let source = "SELECT 1";
    let tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    // Feed SELECT token.
    let r = cursor.feed_token(TokenType::SELECT, 0..6).unwrap();
    assert!(r.is_none());

    // Feed integer literal token.
    let r = cursor.feed_token(TokenType::INTEGER, 7..8).unwrap();
    assert!(r.is_none());

    // finish() synthesizes SEMI + EOF, triggering the ecmd reduction.
    let stmt = cursor.finish().unwrap().expect("expected a statement");
    assert!(matches!(stmt, Stmt::SelectStmt(_)));
}

/// Feed tokens with an explicit SEMI. The LALR(1) parser needs one token of
/// lookahead after SEMI — the statement completes on finish() (which sends EOF).
#[test]
fn feed_tokens_with_semicolon() {
    let source = "SELECT 1;";
    let tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    cursor.feed_token(TokenType::SELECT, 0..6).unwrap();
    cursor.feed_token(TokenType::INTEGER, 7..8).unwrap();

    // SEMI is shifted but the ecmd reduction hasn't fired yet (needs lookahead).
    let r = cursor.feed_token(TokenType::SEMI, 8..9).unwrap();
    assert!(r.is_none());

    // finish() sends EOF which provides the lookahead.
    let stmt = cursor.finish().unwrap().expect("expected a statement");
    assert!(matches!(stmt, Stmt::SelectStmt(_)));
}

/// Multiple statements: the second statement's first token triggers
/// completion of the first statement.
#[test]
fn feed_tokens_multi_statement() {
    let source = "SELECT 1; SELECT 2";
    let tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    // First statement: SELECT 1 ;
    cursor.feed_token(TokenType::SELECT, 0..6).unwrap();
    cursor.feed_token(TokenType::INTEGER, 7..8).unwrap();
    let r = cursor.feed_token(TokenType::SEMI, 8..9).unwrap();
    assert!(r.is_none()); // SEMI shifted, not reduced yet.

    // Second statement's first token provides the lookahead that completes stmt 1.
    let stmt1 = cursor.feed_token(TokenType::SELECT, 10..16).unwrap();
    assert!(
        stmt1.is_some(),
        "first statement should complete on next SELECT"
    );

    // Continue second statement.
    cursor.feed_token(TokenType::INTEGER, 17..18).unwrap();

    let stmt2 = cursor.finish().unwrap();
    assert!(stmt2.is_some(), "second statement should complete");
}

/// TK_SPACE should be silently ignored.
#[test]
fn feed_token_skips_space() {
    let source = "SELECT 1";
    let tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    cursor.feed_token(TokenType::SELECT, 0..6).unwrap();

    // Feed a space — should be silently skipped.
    let r = cursor.feed_token(TokenType::SPACE, 6..7).unwrap();
    assert!(r.is_none());

    cursor.feed_token(TokenType::INTEGER, 7..8).unwrap();

    let stmt = cursor.finish().unwrap().expect("expected a statement");
    assert!(matches!(stmt, Stmt::SelectStmt(_)));
}

/// TK_COMMENT should be recorded as a comment.
#[test]
fn feed_token_records_comment() {
    let source = "SELECT -- hello\n1";
    let tp = IncrementalParser::new(); // collect_tokens is true by default
    let mut cursor = tp.feed(source);

    cursor.feed_token(TokenType::SELECT, 0..6).unwrap();
    cursor.feed_token(TokenType::COMMENT, 7..15).unwrap();
    cursor.feed_token(TokenType::INTEGER, 16..17).unwrap();

    cursor.finish().unwrap().expect("expected a statement");

    let comments = cursor.comments();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].length, 8);
}

/// begin_macro / end_macro records macro regions.
#[test]
fn macro_regions_recorded() {
    let source = "SELECT 1";
    let tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    cursor.begin_macro(7..7 + 13);
    cursor.feed_token(TokenType::SELECT, 0..6).unwrap();
    cursor.feed_token(TokenType::INTEGER, 7..8).unwrap();
    cursor.end_macro();

    cursor.finish().unwrap();

    let regions = cursor.macro_regions();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].call_offset, 7);
    assert_eq!(regions[0].call_length, 13);
}

/// Nested macro regions are both recorded.
#[test]
fn nested_macro_regions() {
    let source = "SELECT 1";
    let tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    cursor.begin_macro(0..0 + 30);
    cursor.begin_macro(10..10 + 5);
    cursor.feed_token(TokenType::SELECT, 0..6).unwrap();
    cursor.feed_token(TokenType::INTEGER, 7..8).unwrap();
    cursor.end_macro();
    cursor.end_macro();

    cursor.finish().unwrap();

    let regions = cursor.macro_regions();
    assert_eq!(regions.len(), 2);
    assert_eq!(regions[0].call_offset, 0);
    assert_eq!(regions[0].call_length, 30);
    assert_eq!(regions[1].call_offset, 10);
    assert_eq!(regions[1].call_length, 5);
}

/// A macro that expands to a complete expression (single node) is well-aligned.
/// The parser should accept it without error.
#[test]
fn macro_well_aligned_complete_expression() {
    let source = "SELECT foo!(1 + 2), 3";
    let tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    cursor.feed_token(TokenType::SELECT, 0..6).unwrap();

    cursor.begin_macro(7..7 + 11);
    cursor.feed_token(TokenType::INTEGER, 12..13).unwrap();
    cursor.feed_token(TokenType::PLUS, 14..15).unwrap();
    cursor.feed_token(TokenType::INTEGER, 16..17).unwrap();
    cursor.end_macro();

    cursor.feed_token(TokenType::COMMA, 18..19).unwrap();
    cursor.feed_token(TokenType::INTEGER, 20..21).unwrap();

    let stmt = cursor.finish().unwrap().expect("expected a statement");
    assert!(matches!(stmt, Stmt::SelectStmt(_)));
}

/// A macro whose expanded tokens end up in a node that also contains
/// tokens from outside the macro region. The parser detects this straddle
/// and returns an error.
#[test]
fn macro_straddle_rejected_by_parser() {
    let source = "SELECT 1 FROM foo!(x) y";
    let tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    cursor.feed_token(TokenType::SELECT, 0..6).unwrap();
    cursor.feed_token(TokenType::INTEGER, 7..8).unwrap();
    cursor.feed_token(TokenType::FROM, 9..13).unwrap();

    cursor.begin_macro(14..14 + 7);
    cursor.feed_token(TokenType::ID, 19..20).unwrap();
    cursor.end_macro();

    cursor.feed_token(TokenType::ID, 22..23).unwrap();
    let err = cursor.finish().unwrap_err();
    assert!(
        err.message.contains("straddle"),
        "expected straddle error, got: {}",
        err.message
    );
}

/// finish() without feeding any tokens returns None.
#[test]
fn finish_with_no_tokens() {
    let source = "";
    let tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    let r = cursor.finish().unwrap();
    assert!(r.is_none());
}

/// High-level API still works after the refactor.
#[test]
fn high_level_api_still_works() {
    let parser = syntaqlite_parser::Parser::new(syntaqlite::dialect::sqlite());
    let mut cursor = parser.parse("SELECT 1; SELECT 2");

    let node1 = cursor.next_statement().unwrap().unwrap();
    assert!(matches!(
        node1.as_typed::<Stmt>().unwrap(),
        Stmt::SelectStmt(_)
    ));

    let node2 = cursor.next_statement().unwrap().unwrap();
    assert!(matches!(
        node2.as_typed::<Stmt>().unwrap(),
        Stmt::SelectStmt(_)
    ));

    assert!(cursor.next_statement().is_none());
}

/// Type names in SQLite type contexts should be marked with AS_TYPE so
/// semantic highlighting can render them as `type`.
#[test]
fn sqlite_type_tokens_are_marked_as_type() {
    use syntaqlite_parser::{Parser, TOKEN_FLAG_AS_TYPE};

    let source = "CREATE TABLE t(a int, b TEXT); SELECT CAST(a AS varchar(10)) FROM t";
    let dialect = syntaqlite::dialect::sqlite();
    let parser = Parser::with_config(
        dialect,
        &syntaqlite_parser::ParserConfig {
            collect_tokens: true,
            ..syntaqlite_parser::ParserConfig::default()
        },
    );
    let mut cursor = parser.parse(source);

    while let Some(stmt) = cursor.next_statement() {
        stmt.expect("parse should succeed");
    }

    let marked: Vec<&str> = cursor
        .tokens()
        .iter()
        .filter(|tp| tp.flags & TOKEN_FLAG_AS_TYPE != 0)
        .map(|tp| &source[tp.offset as usize..(tp.offset as usize + tp.length as usize)])
        .collect();

    assert_eq!(marked, vec!["int", "TEXT", "varchar"]);
}

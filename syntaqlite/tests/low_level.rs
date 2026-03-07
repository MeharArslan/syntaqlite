// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use syntaqlite::nodes::Stmt;
use syntaqlite::{ParseOutcome, Parser, ParserConfig, TokenType};

/// Feed tokens for "SELECT 1" via the low-level API and verify same AST
/// as the high-level parse.
#[test]
fn feed_tokens_select_1() {
    let source = "SELECT 1";
    let parser = Parser::new();
    let mut session = parser.incremental_parse(source);

    // Feed SELECT token.
    assert!(session.feed_token(TokenType::Select, 0..6).is_none());

    // Feed integer literal token.
    assert!(session.feed_token(TokenType::Integer, 7..8).is_none());

    // finish() synthesizes SEMI + EOF, triggering the ecmd reduction.
    let stmt = session.finish().unwrap().expect("expected a statement");
    assert!(matches!(stmt.root(), Stmt::SelectStmt(_)));
}

/// Feed tokens with an explicit SEMI. The LALR(1) parser needs one token of
/// lookahead after SEMI — the statement completes on finish() (which sends EOF).
#[test]
fn feed_tokens_with_semicolon() {
    let source = "SELECT 1;";
    let parser = Parser::new();
    let mut session = parser.incremental_parse(source);

    session.feed_token(TokenType::Select, 0..6);
    session.feed_token(TokenType::Integer, 7..8);

    // SEMI is shifted but the ecmd reduction hasn't fired yet (needs lookahead).
    assert!(session.feed_token(TokenType::Semi, 8..9).is_none());

    // finish() sends EOF which provides the lookahead.
    let stmt = session.finish().unwrap().expect("expected a statement");
    assert!(matches!(stmt.root(), Stmt::SelectStmt(_)));
}

/// Multiple statements: the second statement's first token triggers
/// completion of the first statement.
#[test]
fn feed_tokens_multi_statement() {
    let source = "SELECT 1; SELECT 2";
    let parser = Parser::new();
    let mut session = parser.incremental_parse(source);

    // First statement: SELECT 1 ;
    session.feed_token(TokenType::Select, 0..6);
    session.feed_token(TokenType::Integer, 7..8);
    assert!(session.feed_token(TokenType::Semi, 8..9).is_none()); // SEMI shifted, not reduced yet.

    // Second statement's first token provides the lookahead that completes stmt 1.
    let stmt1 = session.feed_token(TokenType::Select, 10..16);
    assert!(
        stmt1.is_some(),
        "first statement should complete on next SELECT"
    );

    // Continue second statement.
    session.feed_token(TokenType::Integer, 17..18);

    assert!(
        session.finish().is_some(),
        "second statement should complete"
    );
}

/// TK_SPACE should be silently ignored.
#[test]
fn feed_token_skips_space() {
    let source = "SELECT 1";
    let parser = Parser::new();
    let mut session = parser.incremental_parse(source);

    session.feed_token(TokenType::Select, 0..6);

    // Feed a space — should be silently skipped.
    assert!(session.feed_token(TokenType::Space, 6..7).is_none());

    session.feed_token(TokenType::Integer, 7..8);

    let stmt = session.finish().unwrap().expect("expected a statement");
    assert!(matches!(stmt.root(), Stmt::SelectStmt(_)));
}

/// TK_COMMENT should be recorded as a comment.
#[test]
fn feed_token_records_comment() {
    let source = "SELECT -- hello\n1";
    let parser = Parser::with_config(&ParserConfig::default().with_collect_tokens(true));
    let mut session = parser.incremental_parse(source);

    session.feed_token(TokenType::Select, 0..6);
    session.feed_token(TokenType::Comment, 7..15);
    session.feed_token(TokenType::Integer, 16..17);

    let stmt = session.finish().unwrap().expect("expected a statement");

    let comments: Vec<_> = stmt.comments().collect();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].length(), 8);
}

/// begin_macro / end_macro records macro regions.
#[test]
fn macro_regions_recorded() {
    let source = "SELECT 1";
    let parser = Parser::new();
    let mut session = parser.incremental_parse(source);

    session.begin_macro(7..7 + 13);
    session.feed_token(TokenType::Select, 0..6);
    session.feed_token(TokenType::Integer, 7..8);
    session.end_macro();

    let stmt = session.finish().unwrap().expect("expected a statement");

    let regions: Vec<_> = stmt.erase().macro_regions().collect();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].call_offset, 7);
    assert_eq!(regions[0].call_length, 13);
}

/// Nested macro regions are both recorded.
#[test]
fn nested_macro_regions() {
    let source = "SELECT 1";
    let parser = Parser::new();
    let mut session = parser.incremental_parse(source);

    session.begin_macro(0..0 + 30);
    session.begin_macro(10..10 + 5);
    session.feed_token(TokenType::Select, 0..6);
    session.feed_token(TokenType::Integer, 7..8);
    session.end_macro();
    session.end_macro();

    let stmt = session.finish().unwrap().expect("expected a statement");

    let regions: Vec<_> = stmt.erase().macro_regions().collect();
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
    let parser = Parser::new();
    let mut session = parser.incremental_parse(source);

    session.feed_token(TokenType::Select, 0..6);

    session.begin_macro(7..7 + 11);
    session.feed_token(TokenType::Integer, 12..13);
    session.feed_token(TokenType::Plus, 14..15);
    session.feed_token(TokenType::Integer, 16..17);
    session.end_macro();

    session.feed_token(TokenType::Comma, 18..19);
    session.feed_token(TokenType::Integer, 20..21);

    let stmt = session.finish().unwrap().expect("expected a statement");
    assert!(matches!(stmt.root(), Stmt::SelectStmt(_)));
}

/// A macro whose expanded tokens straddle a node boundary: the schema part of
/// `schema.table` comes from inside the macro but `table` is outside. This
/// produces a `TableRef` with `schema` inside the macro and `table_name`
/// outside — a genuine straddle that the parser must reject.
#[test]
fn macro_straddle_rejected_by_parser() {
    // source layout: "SELECT 1 FROM foo!(s).t"
    //                 0      7 9    14     21 22
    //  macro region: 14..21 covers "foo!(s)"
    //  schema token: Id at 19..20 (the 's', inside macro)
    //  dot:          Dot at 21..22 (outside macro)
    //  table token:  Id at 22..23 (the 't', outside macro)
    let source = "SELECT 1 FROM foo!(s).t";
    let parser = Parser::new();
    let mut session = parser.incremental_parse(source);

    session.feed_token(TokenType::Select, 0..6);
    session.feed_token(TokenType::Integer, 7..8);
    session.feed_token(TokenType::From, 9..13);

    session.begin_macro(14..14 + 7); // foo!(s) = 7 chars
    session.feed_token(TokenType::Id, 19..20); // schema 's', inside macro
    session.end_macro();

    session.feed_token(TokenType::Dot, 21..22);
    session.feed_token(TokenType::Id, 22..23); // table name 't', outside macro

    let result = session.finish().expect("should return Some");
    let Err(err) = result else {
        panic!("expected straddle error but parse succeeded");
    };
    assert!(
        err.message().contains("straddle"),
        "expected straddle error, got: {}",
        err.message()
    );
}

/// finish() without feeding any tokens returns None.
#[test]
fn finish_with_no_tokens() {
    let source = "";
    let parser = Parser::new();
    let mut session = parser.incremental_parse(source);

    assert!(session.finish().is_none());
}

/// High-level API still works after the refactor.
#[test]
fn high_level_api_still_works() {
    let parser = Parser::new();
    let mut session = parser.parse("SELECT 1; SELECT 2");

    let ParseOutcome::Ok(stmt1) = session.next() else {
        panic!("expected Ok")
    };
    assert!(matches!(stmt1.root(), Stmt::SelectStmt(_)));

    let ParseOutcome::Ok(stmt2) = session.next() else {
        panic!("expected Ok")
    };
    assert!(matches!(stmt2.root(), Stmt::SelectStmt(_)));

    assert!(matches!(session.next(), ParseOutcome::Done));
}

/// Type names in SQLite type contexts should be marked with AS_TYPE so
/// semantic highlighting can render them as `type`.
#[test]
fn sqlite_type_tokens_are_marked_as_type() {
    let source = "CREATE TABLE t(a int, b TEXT); SELECT CAST(a AS varchar(10)) FROM t";
    let parser = Parser::with_config(&ParserConfig::default().with_collect_tokens(true));
    let mut session = parser.parse(source);

    let mut marked = Vec::new();
    loop {
        match session.next() {
            ParseOutcome::Ok(stmt) => {
                for t in stmt.tokens() {
                    if t.flags().used_as_type() {
                        marked.push(
                            source[t.offset() as usize..(t.offset() + t.length()) as usize]
                                .to_string(),
                        );
                    }
                }
            }
            ParseOutcome::Err(e) => panic!("parse error: {}", e.message()),
            ParseOutcome::Done => break,
        }
    }

    assert_eq!(marked, vec!["int", "TEXT", "varchar"]);
}

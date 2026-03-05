// Copyright 2026 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![cfg(feature = "sqlite")]
#![allow(missing_docs)]

use syntaqlite_syntax::{CommentKind, ParseErrorKind, Parser, ParserConfig, TokenType, Tokenizer};

#[test]
fn tokenizing_example_breaks_source_into_tokens() {
    let tokenizer = Tokenizer::new();
    let tokens: Vec<_> = tokenizer
        .tokenize("SELECT 1")
        .map(|token| (token.token_type(), token.text().to_owned()))
        .collect();

    assert_eq!(
        tokens,
        vec![
            (TokenType::Select, "SELECT".to_owned()),
            (TokenType::Space, " ".to_owned()),
            (TokenType::Integer, "1".to_owned()),
        ]
    );
}

#[test]
fn parsing_example_loop_yields_successful_statement() {
    let parser = Parser::new();
    let mut session = parser.parse("SELECT 1");

    let mut ok_count = 0usize;
    while let Some(statement) = session.next() {
        match statement {
            Ok(statement) => {
                ok_count += 1;
                assert!(statement.root().is_some());
            }
            Err(error) => {
                assert!(!error.message().is_empty());
                if error.kind() == ParseErrorKind::Fatal {
                    break;
                }
            }
        }
    }

    assert_eq!(ok_count, 1);
}

#[test]
fn parser_can_be_reused_across_multiple_inputs() {
    let parser = Parser::new();

    let mut first = parser.parse("SELECT 1");
    let first_stmt = first
        .next()
        .expect("first parse should produce a statement");
    assert!(matches!(first_stmt, Ok(statement) if statement.root().is_some()));
    assert!(first.next().is_none());
    drop(first);

    let mut second = parser.parse("SELECT 2");
    let second_stmt = second
        .next()
        .expect("second parse should produce a statement");
    assert!(matches!(second_stmt, Ok(statement) if statement.root().is_some()));
    assert!(second.next().is_none());
}

#[test]
fn parse_session_continues_after_statement_error() {
    let parser = Parser::new();
    let mut session = parser.parse("SELECT 1; SELECT ; SELECT 2;");

    let first = session.next().expect("first statement should exist");
    assert!(matches!(first, Ok(statement) if statement.root().is_some()));

    let second = session.next().expect("second statement should exist");
    let error = match second {
        Ok(_) => panic!("second statement should fail"),
        Err(error) => error,
    };
    assert!(!error.message().is_empty());
    assert!(matches!(
        error.kind(),
        ParseErrorKind::Recovered | ParseErrorKind::Fatal
    ));

    let third = session
        .next()
        .expect("session should continue to next statement");
    assert!(matches!(third, Ok(statement) if statement.root().is_some()));
    assert!(session.next().is_none());
}

#[test]
fn collect_tokens_and_comments_follows_parser_config() {
    let mut disabled = Parser::new().parse("/* lead */ SELECT 1 -- tail\n;");
    let disabled_stmt = disabled
        .next()
        .expect("statement should exist")
        .expect("statement should parse");
    assert_eq!(disabled_stmt.tokens().count(), 0);
    assert_eq!(disabled_stmt.comments().count(), 0);

    let parser = Parser::with_config(&ParserConfig::default().with_collect_tokens(true));
    let mut enabled = parser.parse("/* lead */ SELECT 1 -- tail\n;");
    let enabled_stmt = enabled
        .next()
        .expect("statement should exist")
        .expect("statement should parse");

    let token_types: Vec<_> = enabled_stmt
        .tokens()
        .map(|token| token.token_type())
        .collect();
    assert!(token_types.contains(&TokenType::Select));
    assert!(token_types.contains(&TokenType::Integer));

    let comments: Vec<_> = enabled_stmt.comments().collect();
    assert!(
        comments
            .iter()
            .any(|comment| comment.kind == CommentKind::Block && comment.text.contains("lead"))
    );
    assert!(
        comments
            .iter()
            .any(|comment| comment.kind == CommentKind::Line && comment.text.contains("tail"))
    );
}

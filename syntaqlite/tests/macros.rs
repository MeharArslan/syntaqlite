// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

/// Integration tests: macro regions are emitted verbatim by the formatter.
use syntaqlite::low_level::TokenParser;
use syntaqlite::low_level::TokenType;

fn runtime_formatter() -> syntaqlite_runtime::fmt::Formatter<'static> {
    syntaqlite_runtime::fmt::Formatter::new(syntaqlite::low_level::Sqlite::dialect()).unwrap()
}

#[test]
fn macro_call_emitted_verbatim() {
    let source = "SELECT foo!(1 + 2), 3";
    let fmt = runtime_formatter();

    let mut tp = TokenParser::new();
    let mut feeder = tp.feed(source);

    feeder.feed_token(TokenType::Select, 0..6).unwrap();

    feeder.begin_macro(7, 11);
    feeder.feed_token(TokenType::Integer, 12..13).unwrap();
    feeder.feed_token(TokenType::Plus, 14..15).unwrap();
    feeder.feed_token(TokenType::Integer, 16..17).unwrap();
    feeder.end_macro();

    feeder.feed_token(TokenType::Comma, 18..19).unwrap();
    feeder.feed_token(TokenType::Integer, 20..21).unwrap();

    let root = feeder.finish().unwrap().expect("expected a statement");

    assert_eq!(fmt.format_node(feeder.base(), root), "SELECT foo!(1 + 2), 3");
}

#[test]
fn macro_multi_node_emitted_once() {
    let source = "SELECT macro!(a, b)";
    let fmt = runtime_formatter();

    let mut tp = TokenParser::new();
    let mut feeder = tp.feed(source);

    feeder.feed_token(TokenType::Select, 0..6).unwrap();

    feeder.begin_macro(7, 12);
    feeder.feed_token(TokenType::Id, 14..15).unwrap();
    feeder.feed_token(TokenType::Comma, 15..16).unwrap();
    feeder.feed_token(TokenType::Id, 17..18).unwrap();
    feeder.end_macro();

    let root = feeder.finish().unwrap().expect("expected a statement");

    assert_eq!(fmt.format_node(feeder.base(), root), "SELECT macro!(a, b)");
}

#[test]
fn macro_multi_node_no_extra_separator() {
    let source = "SELECT foo!(a, b), c";
    let fmt = runtime_formatter();

    let mut tp = TokenParser::new();
    let mut feeder = tp.feed(source);

    feeder.feed_token(TokenType::Select, 0..6).unwrap();

    feeder.begin_macro(7, 10);
    feeder.feed_token(TokenType::Id, 12..13).unwrap();
    feeder.feed_token(TokenType::Comma, 13..14).unwrap();
    feeder.feed_token(TokenType::Id, 15..16).unwrap();
    feeder.end_macro();

    feeder.feed_token(TokenType::Comma, 17..18).unwrap();
    feeder.feed_token(TokenType::Id, 19..20).unwrap();

    let root = feeder.finish().unwrap().expect("expected a statement");

    assert_eq!(fmt.format_node(feeder.base(), root), "SELECT foo!(a, b), c");
}

#[test]
fn no_macro_regions_formats_normally() {
    let source = "SELECT  1+2,  3";
    let fmt = runtime_formatter();

    let mut tp = TokenParser::new();
    let mut feeder = tp.feed(source);

    feeder.feed_token(TokenType::Select, 0..6).unwrap();
    feeder.feed_token(TokenType::Integer, 8..9).unwrap();
    feeder.feed_token(TokenType::Plus, 9..10).unwrap();
    feeder.feed_token(TokenType::Integer, 10..11).unwrap();
    feeder.feed_token(TokenType::Comma, 11..12).unwrap();
    feeder.feed_token(TokenType::Integer, 14..15).unwrap();

    let root = feeder.finish().unwrap().expect("expected a statement");

    assert_eq!(fmt.format_node(feeder.base(), root), "SELECT 1 + 2, 3");
}
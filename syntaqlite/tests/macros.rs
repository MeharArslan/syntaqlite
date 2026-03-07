// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

/// Integration tests: macro regions are emitted verbatim by the formatter.
use syntaqlite::{Parser, ParserConfig};

fn formatter() -> syntaqlite::Formatter {
    syntaqlite::Formatter::new()
}

/// Parser configured for use with `format_parsed`: tokens must be collected
/// so the formatter can detect macro region boundaries.
fn parser() -> Parser {
    Parser::with_config(&ParserConfig::default().with_collect_tokens(true))
}

mod tk {
    use syntaqlite::TokenType;
    pub(crate) const SELECT: TokenType = TokenType::Select;
    pub(crate) const INTEGER: TokenType = TokenType::Integer;
    pub(crate) const PLUS: TokenType = TokenType::Plus;
    pub(crate) const COMMA: TokenType = TokenType::Comma;
    pub(crate) const ID: TokenType = TokenType::Id;
}

#[test]
fn macro_call_emitted_verbatim() {
    let source = "SELECT foo!(1 + 2), 3";
    let mut fmt = formatter();

    let parser = parser();
    let mut cursor = parser.incremental_parse(source);

    cursor.feed_token(tk::SELECT, 0..6);

    cursor.begin_macro(7..7 + 11);
    cursor.feed_token(tk::INTEGER, 12..13);
    cursor.feed_token(tk::PLUS, 14..15);
    cursor.feed_token(tk::INTEGER, 16..17);
    cursor.end_macro();

    cursor.feed_token(tk::COMMA, 18..19);
    cursor.feed_token(tk::INTEGER, 20..21);

    let stmt = cursor.finish().unwrap().expect("expected a statement");

    assert_eq!(fmt.format_parsed(stmt.erase()), "SELECT foo!(1 + 2), 3");
}

#[test]
fn macro_multi_node_emitted_once() {
    let source = "SELECT macro!(a, b)";
    let mut fmt = formatter();

    let parser = parser();
    let mut cursor = parser.incremental_parse(source);

    cursor.feed_token(tk::SELECT, 0..6);

    cursor.begin_macro(7..7 + 12);
    cursor.feed_token(tk::ID, 14..15);
    cursor.feed_token(tk::COMMA, 15..16);
    cursor.feed_token(tk::ID, 17..18);
    cursor.end_macro();

    let stmt = cursor.finish().unwrap().expect("expected a statement");

    assert_eq!(fmt.format_parsed(stmt.erase()), "SELECT macro!(a, b)");
}

#[test]
fn macro_multi_node_no_extra_separator() {
    let source = "SELECT foo!(a, b), c";
    let mut fmt = formatter();

    let parser = parser();
    let mut cursor = parser.incremental_parse(source);

    cursor.feed_token(tk::SELECT, 0..6);

    cursor.begin_macro(7..7 + 10);
    cursor.feed_token(tk::ID, 12..13);
    cursor.feed_token(tk::COMMA, 13..14);
    cursor.feed_token(tk::ID, 15..16);
    cursor.end_macro();

    cursor.feed_token(tk::COMMA, 17..18);
    cursor.feed_token(tk::ID, 19..20);

    let stmt = cursor.finish().unwrap().expect("expected a statement");

    assert_eq!(fmt.format_parsed(stmt.erase()), "SELECT foo!(a, b), c");
}

#[test]
fn no_macro_regions_formats_normally() {
    let source = "SELECT  1+2,  3";
    let mut fmt = formatter();

    let parser = parser();
    let mut cursor = parser.incremental_parse(source);

    cursor.feed_token(tk::SELECT, 0..6);
    cursor.feed_token(tk::INTEGER, 8..9);
    cursor.feed_token(tk::PLUS, 9..10);
    cursor.feed_token(tk::INTEGER, 10..11);
    cursor.feed_token(tk::COMMA, 11..12);
    cursor.feed_token(tk::INTEGER, 14..15);

    let stmt = cursor.finish().unwrap().expect("expected a statement");

    assert_eq!(fmt.format_parsed(stmt.erase()), "SELECT 1 + 2, 3");
}

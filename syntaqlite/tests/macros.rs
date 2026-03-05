// TODO: broken - needs migration to syntaqlite_syntax
#![cfg(broken_needs_migration)]

// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

/// Integration tests: macro regions are emitted verbatim by the formatter.
use syntaqlite::incremental::IncrementalParser;

fn formatter() -> syntaqlite::Formatter<'static> {
    syntaqlite::Formatter::new()
}

mod tk {
    use syntaqlite::TokenType;
    pub(crate) const SELECT: TokenType = TokenType::SELECT;
    pub(crate) const INTEGER: TokenType = TokenType::INTEGER;
    pub(crate) const PLUS: TokenType = TokenType::PLUS;
    pub(crate) const COMMA: TokenType = TokenType::COMMA;
    pub(crate) const ID: TokenType = TokenType::ID;
}

#[test]
fn macro_call_emitted_verbatim() {
    let source = "SELECT foo!(1 + 2), 3";
    let mut fmt = formatter();

    let tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    cursor.feed_token(tk::SELECT, 0..6).unwrap();

    cursor.begin_macro(7..7 + 11);
    cursor.feed_token(tk::INTEGER, 12..13).unwrap();
    cursor.feed_token(tk::PLUS, 14..15).unwrap();
    cursor.feed_token(tk::INTEGER, 16..17).unwrap();
    cursor.end_macro();

    cursor.feed_token(tk::COMMA, 18..19).unwrap();
    cursor.feed_token(tk::INTEGER, 20..21).unwrap();

    cursor.finish().unwrap().expect("expected a statement");

    assert_eq!(
        fmt.format_node(cursor.root().unwrap()),
        "SELECT foo!(1 + 2), 3"
    );
}

#[test]
fn macro_multi_node_emitted_once() {
    let source = "SELECT macro!(a, b)";
    let mut fmt = formatter();

    let tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    cursor.feed_token(tk::SELECT, 0..6).unwrap();

    cursor.begin_macro(7..7 + 12);
    cursor.feed_token(tk::ID, 14..15).unwrap();
    cursor.feed_token(tk::COMMA, 15..16).unwrap();
    cursor.feed_token(tk::ID, 17..18).unwrap();
    cursor.end_macro();

    cursor.finish().unwrap().expect("expected a statement");

    assert_eq!(
        fmt.format_node(cursor.root().unwrap()),
        "SELECT macro!(a, b)"
    );
}

#[test]
fn macro_multi_node_no_extra_separator() {
    let source = "SELECT foo!(a, b), c";
    let mut fmt = formatter();

    let tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    cursor.feed_token(tk::SELECT, 0..6).unwrap();

    cursor.begin_macro(7..7 + 10);
    cursor.feed_token(tk::ID, 12..13).unwrap();
    cursor.feed_token(tk::COMMA, 13..14).unwrap();
    cursor.feed_token(tk::ID, 15..16).unwrap();
    cursor.end_macro();

    cursor.feed_token(tk::COMMA, 17..18).unwrap();
    cursor.feed_token(tk::ID, 19..20).unwrap();

    cursor.finish().unwrap().expect("expected a statement");

    assert_eq!(
        fmt.format_node(cursor.root().unwrap()),
        "SELECT foo!(a, b), c"
    );
}

#[test]
fn no_macro_regions_formats_normally() {
    let source = "SELECT  1+2,  3";
    let mut fmt = formatter();

    let tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    cursor.feed_token(tk::SELECT, 0..6).unwrap();
    cursor.feed_token(tk::INTEGER, 8..9).unwrap();
    cursor.feed_token(tk::PLUS, 9..10).unwrap();
    cursor.feed_token(tk::INTEGER, 10..11).unwrap();
    cursor.feed_token(tk::COMMA, 11..12).unwrap();
    cursor.feed_token(tk::INTEGER, 14..15).unwrap();

    cursor.finish().unwrap().expect("expected a statement");

    assert_eq!(fmt.format_node(cursor.root().unwrap()), "SELECT 1 + 2, 3");
}

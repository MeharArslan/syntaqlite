// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

/// Integration tests: macro regions are emitted verbatim by the formatter.
use syntaqlite_runtime::parser::LowLevelParser;

fn dialect() -> &'static syntaqlite_runtime::Dialect<'static> {
    syntaqlite::low_level::dialect()
}

fn formatter() -> syntaqlite_runtime::fmt::Formatter<'static> {
    syntaqlite_runtime::fmt::Formatter::new(dialect()).unwrap()
}

// Token type constants (raw u32 values for the runtime API).
mod tk {
    use syntaqlite::low_level::TokenType;
    pub const SELECT: u32 = TokenType::Select as u32;
    pub const INTEGER: u32 = TokenType::Integer as u32;
    pub const PLUS: u32 = TokenType::Plus as u32;
    pub const COMMA: u32 = TokenType::Comma as u32;
    pub const ID: u32 = TokenType::Id as u32;
}

#[test]
fn macro_call_emitted_verbatim() {
    let source = "SELECT foo!(1 + 2), 3";
    let fmt = formatter();

    let mut tp = LowLevelParser::new(dialect());
    let mut cursor = tp.feed(source);

    cursor.feed_token(tk::SELECT, 0..6).unwrap();

    cursor.begin_macro(7, 11);
    cursor.feed_token(tk::INTEGER, 12..13).unwrap();
    cursor.feed_token(tk::PLUS, 14..15).unwrap();
    cursor.feed_token(tk::INTEGER, 16..17).unwrap();
    cursor.end_macro();

    cursor.feed_token(tk::COMMA, 18..19).unwrap();
    cursor.feed_token(tk::INTEGER, 20..21).unwrap();

    let root = cursor.finish().unwrap().expect("expected a statement");

    assert_eq!(fmt.format_node(cursor.base(), root), "SELECT foo!(1 + 2), 3");
}

#[test]
fn macro_multi_node_emitted_once() {
    let source = "SELECT macro!(a, b)";
    let fmt = formatter();

    let mut tp = LowLevelParser::new(dialect());
    let mut cursor = tp.feed(source);

    cursor.feed_token(tk::SELECT, 0..6).unwrap();

    cursor.begin_macro(7, 12);
    cursor.feed_token(tk::ID, 14..15).unwrap();
    cursor.feed_token(tk::COMMA, 15..16).unwrap();
    cursor.feed_token(tk::ID, 17..18).unwrap();
    cursor.end_macro();

    let root = cursor.finish().unwrap().expect("expected a statement");

    assert_eq!(fmt.format_node(cursor.base(), root), "SELECT macro!(a, b)");
}

#[test]
fn macro_multi_node_no_extra_separator() {
    let source = "SELECT foo!(a, b), c";
    let fmt = formatter();

    let mut tp = LowLevelParser::new(dialect());
    let mut cursor = tp.feed(source);

    cursor.feed_token(tk::SELECT, 0..6).unwrap();

    cursor.begin_macro(7, 10);
    cursor.feed_token(tk::ID, 12..13).unwrap();
    cursor.feed_token(tk::COMMA, 13..14).unwrap();
    cursor.feed_token(tk::ID, 15..16).unwrap();
    cursor.end_macro();

    cursor.feed_token(tk::COMMA, 17..18).unwrap();
    cursor.feed_token(tk::ID, 19..20).unwrap();

    let root = cursor.finish().unwrap().expect("expected a statement");

    assert_eq!(fmt.format_node(cursor.base(), root), "SELECT foo!(a, b), c");
}

#[test]
fn no_macro_regions_formats_normally() {
    let source = "SELECT  1+2,  3";
    let fmt = formatter();

    let mut tp = LowLevelParser::new(dialect());
    let mut cursor = tp.feed(source);

    cursor.feed_token(tk::SELECT, 0..6).unwrap();
    cursor.feed_token(tk::INTEGER, 8..9).unwrap();
    cursor.feed_token(tk::PLUS, 9..10).unwrap();
    cursor.feed_token(tk::INTEGER, 10..11).unwrap();
    cursor.feed_token(tk::COMMA, 11..12).unwrap();
    cursor.feed_token(tk::INTEGER, 14..15).unwrap();

    let root = cursor.finish().unwrap().expect("expected a statement");

    assert_eq!(fmt.format_node(cursor.base(), root), "SELECT 1 + 2, 3");
}

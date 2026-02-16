/// Integration tests: macro regions are emitted verbatim by the formatter.
use syntaqlite_fmt::generated::fmt_ops::{CTX, DISPATCH};
use syntaqlite_fmt::{format_node, render, DocArena, FormatConfig};
use syntaqlite_parser::{Parser, TokenType};

/// Simulate a macro call "foo!(1 + 2)" that expands to tokens 1, +, 2.
/// The formatter should emit the macro call text verbatim.
///
/// Source layout: "SELECT foo!(1 + 2), 3"
///                 0         1         2
///                 0123456789012345678901
/// Macro region:         ^----------^  call_offset=7, call_length=11
///                       "foo!(1 + 2)"
///
/// The macro expands to the expression "1 + 2" — we feed INTEGER, PLUS, INTEGER
/// with text pointers into the original source within the macro region.
#[test]
fn macro_call_emitted_verbatim() {
    let source = "SELECT foo!(1 + 2), 3";

    let mut parser = Parser::new();
    let mut session = parser.parse(source);

    // Feed SELECT (offset 0..6).
    session
        .feed_token(TokenType::Select as u32, &source[0..6])
        .unwrap();

    // Begin macro region: "foo!(1 + 2)" at offset 7, length 11.
    session.begin_macro(7, 11);

    // Feed the expanded tokens. Their text pointers are within the macro call
    // region in the source string.
    session
        .feed_token(TokenType::Integer as u32, &source[12..13]) // "1"
        .unwrap();
    session
        .feed_token(TokenType::Plus as u32, &source[14..15]) // "+"
        .unwrap();
    session
        .feed_token(TokenType::Integer as u32, &source[16..17]) // "2"
        .unwrap();

    session.end_macro();

    // Feed COMMA and "3".
    session
        .feed_token(TokenType::Comma as u32, &source[18..19])
        .unwrap();
    session
        .feed_token(TokenType::Integer as u32, &source[20..21])
        .unwrap();

    let root = session.finish().unwrap().expect("expected a statement");

    let mut arena = DocArena::new();
    let doc = format_node(&DISPATCH, &CTX, &session, root, &mut arena);
    let result = render(&arena, doc, &FormatConfig::default());

    assert_eq!(result, "SELECT foo!(1 + 2), 3");
}

/// When there are no macro regions, formatting proceeds normally.
#[test]
fn no_macro_regions_formats_normally() {
    let source = "SELECT  1+2,  3";

    let mut parser = Parser::new();
    let mut session = parser.parse(source);

    session
        .feed_token(TokenType::Select as u32, &source[0..6])
        .unwrap();
    session
        .feed_token(TokenType::Integer as u32, &source[8..9])
        .unwrap();
    session
        .feed_token(TokenType::Plus as u32, &source[9..10])
        .unwrap();
    session
        .feed_token(TokenType::Integer as u32, &source[10..11])
        .unwrap();
    session
        .feed_token(TokenType::Comma as u32, &source[11..12])
        .unwrap();
    session
        .feed_token(TokenType::Integer as u32, &source[14..15])
        .unwrap();

    let root = session.finish().unwrap().expect("expected a statement");

    let mut arena = DocArena::new();
    let doc = format_node(&DISPATCH, &CTX, &session, root, &mut arena);
    let result = render(&arena, doc, &FormatConfig::default());

    // Normal formatting: spaces normalized, no verbatim override.
    assert_eq!(result, "SELECT 1 + 2, 3");
}

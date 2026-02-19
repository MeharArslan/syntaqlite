/// Integration tests: macro regions are emitted verbatim by the formatter.
use syntaqlite_fmt::{ctx, dispatch, format_node, render, DocArena, FormatConfig};
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
        .feed_token(TokenType::Select, &source[0..6])
        .unwrap();

    // Begin macro region: "foo!(1 + 2)" at offset 7, length 11.
    session.begin_macro(7, 11);

    // Feed the expanded tokens. Their text pointers are within the macro call
    // region in the source string.
    session
        .feed_token(TokenType::Integer, &source[12..13]) // "1"
        .unwrap();
    session
        .feed_token(TokenType::Plus, &source[14..15]) // "+"
        .unwrap();
    session
        .feed_token(TokenType::Integer, &source[16..17]) // "2"
        .unwrap();

    session.end_macro();

    // Feed COMMA and "3".
    session
        .feed_token(TokenType::Comma, &source[18..19])
        .unwrap();
    session
        .feed_token(TokenType::Integer, &source[20..21])
        .unwrap();

    let root = session.finish().unwrap().expect("expected a statement");

    let mut arena = DocArena::new();
    let doc = format_node(dispatch(), ctx(), &session, root, &mut arena);
    let result = render(&arena, doc, &FormatConfig::default());

    assert_eq!(result, "SELECT foo!(1 + 2), 3");
}

/// A macro that expands to multiple sibling nodes (two result columns).
/// The macro text should be emitted once, not duplicated per sibling.
///
/// Source: "SELECT macro!(a, b)"
///          0         1
///          0123456789012345678
/// Macro:         ^-----------^  offset=7, length=12 → "macro!(a, b)"
///
/// Expanded tokens: ID "a", COMMA, ID "b" — creates two ResultColumn nodes,
/// both with spans inside the macro region. The formatter must emit the macro
/// call text exactly once, suppressing subsequent contained siblings.
#[test]
fn macro_multi_node_emitted_once() {
    let source = "SELECT macro!(a, b)";

    let mut parser = Parser::new();
    let mut session = parser.parse(source);

    session
        .feed_token(TokenType::Select, &source[0..6])
        .unwrap();

    // Macro: "macro!(a, b)" at offset 7, length 12.
    session.begin_macro(7, 12);
    session
        .feed_token(TokenType::Id, &source[14..15]) // "a"
        .unwrap();
    session
        .feed_token(TokenType::Comma, &source[15..16]) // ","
        .unwrap();
    session
        .feed_token(TokenType::Id, &source[17..18]) // "b"
        .unwrap();
    session.end_macro();

    let root = session.finish().unwrap().expect("expected a statement");

    let mut arena = DocArena::new();
    let doc = format_node(dispatch(), ctx(), &session, root, &mut arena);
    let result = render(&arena, doc, &FormatConfig::default());

    assert_eq!(result, "SELECT macro!(a, b)");
}

/// A macro that expands to multiple sibling list items (two result columns)
/// with a non-macro sibling after. The macro text should be emitted once,
/// suppressed siblings should not produce extra separators.
///
/// Source: "SELECT foo!(a, b), c"
///          0         1
///          0123456789012345678901
///          SELECT foo!(a, b), c
/// Macro:         ^----------^  offset=7, length=12 → "foo!(a, b)"
///
/// Expanded tokens: ID "a", COMMA, ID "b" — creates two ResultColumn nodes.
/// After end_macro: COMMA, ID "c" — a third ResultColumn outside the macro.
/// Expected: "SELECT foo!(a, b), c" (no double comma).
#[test]
fn macro_multi_node_no_extra_separator() {
    let source = "SELECT foo!(a, b), c";

    let mut parser = Parser::new();
    let mut session = parser.parse(source);

    session
        .feed_token(TokenType::Select, &source[0..6])
        .unwrap();

    // Macro: "foo!(a, b)" at offset 7, length 12.
    //   f  o  o  !  (  a  ,     b  )
    //   7  8  9  10 11 12 13 14 15 16
    // So "foo!(a, b)" = source[7..17], but call_length covers through ')' = 10 chars.
    // Wait: "foo!(a, b)" has length 10? f-o-o-!-(-a-,-space-b-) = 10. offset=7, length=10.
    // But the test comment says length=12. Let me recount.
    // source = "SELECT foo!(a, b), c"
    //           0123456789...
    // 'f' is at index 7.
    // "foo!(a, b)" = f(7) o(8) o(9) !(10) ((11) a(12) ,(13) (14) b(15) )(16)
    // That's 10 characters: indices 7..17, length = 10.
    session.begin_macro(7, 10);
    session
        .feed_token(TokenType::Id, &source[12..13]) // "a"
        .unwrap();
    session
        .feed_token(TokenType::Comma, &source[13..14]) // ","
        .unwrap();
    session
        .feed_token(TokenType::Id, &source[15..16]) // "b"
        .unwrap();
    session.end_macro();

    // Non-macro siblings: ", c"
    session
        .feed_token(TokenType::Comma, &source[17..18])
        .unwrap();
    session
        .feed_token(TokenType::Id, &source[19..20]) // "c"
        .unwrap();

    let root = session.finish().unwrap().expect("expected a statement");

    let mut arena = DocArena::new();
    let doc = format_node(dispatch(), ctx(), &session, root, &mut arena);
    let result = render(&arena, doc, &FormatConfig::default());

    assert_eq!(result, "SELECT foo!(a, b), c");
}

/// When there are no macro regions, formatting proceeds normally.
#[test]
fn no_macro_regions_formats_normally() {
    let source = "SELECT  1+2,  3";

    let mut parser = Parser::new();
    let mut session = parser.parse(source);

    session
        .feed_token(TokenType::Select, &source[0..6])
        .unwrap();
    session
        .feed_token(TokenType::Integer, &source[8..9])
        .unwrap();
    session
        .feed_token(TokenType::Plus, &source[9..10])
        .unwrap();
    session
        .feed_token(TokenType::Integer, &source[10..11])
        .unwrap();
    session
        .feed_token(TokenType::Comma, &source[11..12])
        .unwrap();
    session
        .feed_token(TokenType::Integer, &source[14..15])
        .unwrap();

    let root = session.finish().unwrap().expect("expected a statement");

    let mut arena = DocArena::new();
    let doc = format_node(dispatch(), ctx(), &session, root, &mut arena);
    let result = render(&arena, doc, &FormatConfig::default());

    // Normal formatting: spaces normalized, no verbatim override.
    assert_eq!(result, "SELECT 1 + 2, 3");
}

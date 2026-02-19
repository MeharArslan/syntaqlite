/// Integration tests: macro regions are emitted verbatim by the formatter.
use syntaqlite::ast::SessionExt;
use syntaqlite::tokens::TokenType;
use syntaqlite_runtime::fmt::Formatter;

fn formatter() -> Formatter<'static> {
    Formatter::new(syntaqlite::dialect()).unwrap()
}

#[test]
fn macro_call_emitted_verbatim() {
    let source = "SELECT foo!(1 + 2), 3";
    let fmt = formatter();

    let mut parser = syntaqlite::create_parser();
    let mut session = parser.parse(source);

    session.feed(TokenType::Select, &source[0..6]).unwrap();

    session.begin_macro(7, 11);
    session.feed(TokenType::Integer, &source[12..13]).unwrap();
    session.feed(TokenType::Plus, &source[14..15]).unwrap();
    session.feed(TokenType::Integer, &source[16..17]).unwrap();
    session.end_macro();

    session.feed(TokenType::Comma, &source[18..19]).unwrap();
    session.feed(TokenType::Integer, &source[20..21]).unwrap();

    let root = session.finish().unwrap().expect("expected a statement");

    assert_eq!(fmt.format_node(&session, root), "SELECT foo!(1 + 2), 3");
}

#[test]
fn macro_multi_node_emitted_once() {
    let source = "SELECT macro!(a, b)";
    let fmt = formatter();

    let mut parser = syntaqlite::create_parser();
    let mut session = parser.parse(source);

    session.feed(TokenType::Select, &source[0..6]).unwrap();

    session.begin_macro(7, 12);
    session.feed(TokenType::Id, &source[14..15]).unwrap();
    session.feed(TokenType::Comma, &source[15..16]).unwrap();
    session.feed(TokenType::Id, &source[17..18]).unwrap();
    session.end_macro();

    let root = session.finish().unwrap().expect("expected a statement");

    assert_eq!(fmt.format_node(&session, root), "SELECT macro!(a, b)");
}

#[test]
fn macro_multi_node_no_extra_separator() {
    let source = "SELECT foo!(a, b), c";
    let fmt = formatter();

    let mut parser = syntaqlite::create_parser();
    let mut session = parser.parse(source);

    session.feed(TokenType::Select, &source[0..6]).unwrap();

    session.begin_macro(7, 10);
    session.feed(TokenType::Id, &source[12..13]).unwrap();
    session.feed(TokenType::Comma, &source[13..14]).unwrap();
    session.feed(TokenType::Id, &source[15..16]).unwrap();
    session.end_macro();

    session.feed(TokenType::Comma, &source[17..18]).unwrap();
    session.feed(TokenType::Id, &source[19..20]).unwrap();

    let root = session.finish().unwrap().expect("expected a statement");

    assert_eq!(fmt.format_node(&session, root), "SELECT foo!(a, b), c");
}

#[test]
fn no_macro_regions_formats_normally() {
    let source = "SELECT  1+2,  3";
    let fmt = formatter();

    let mut parser = syntaqlite::create_parser();
    let mut session = parser.parse(source);

    session.feed(TokenType::Select, &source[0..6]).unwrap();
    session.feed(TokenType::Integer, &source[8..9]).unwrap();
    session.feed(TokenType::Plus, &source[9..10]).unwrap();
    session.feed(TokenType::Integer, &source[10..11]).unwrap();
    session.feed(TokenType::Comma, &source[11..12]).unwrap();
    session.feed(TokenType::Integer, &source[14..15]).unwrap();

    let root = session.finish().unwrap().expect("expected a statement");

    assert_eq!(fmt.format_node(&session, root), "SELECT 1 + 2, 3");
}

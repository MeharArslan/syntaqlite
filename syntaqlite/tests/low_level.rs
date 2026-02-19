use syntaqlite::ast::NodeTag;
use syntaqlite::tokens::TokenType;

/// Feed tokens for "SELECT 1" via the low-level API and verify same AST
/// as the high-level parse.
#[test]
fn feed_tokens_select_1() {
    let source = "SELECT 1";
    let mut tp = syntaqlite::low_level::TokenParser::new();
    let mut feeder = tp.feed(source);

    // Feed SELECT token.
    let r = feeder.feed_token(TokenType::Select, 0..6).unwrap();
    assert!(r.is_none());

    // Feed integer literal token.
    let r = feeder.feed_token(TokenType::Integer, 7..8).unwrap();
    assert!(r.is_none());

    // finish() synthesizes SEMI + EOF, triggering the ecmd reduction.
    let root_id = feeder.finish().unwrap().expect("expected a statement");
    let node = feeder.node(root_id).unwrap();
    assert_eq!(node.tag(), NodeTag::SelectStmt);
}

/// Feed tokens with an explicit SEMI. The LALR(1) parser needs one token of
/// lookahead after SEMI — the statement completes on finish() (which sends EOF).
#[test]
fn feed_tokens_with_semicolon() {
    let source = "SELECT 1;";
    let mut tp = syntaqlite::low_level::TokenParser::new();
    let mut feeder = tp.feed(source);

    feeder.feed_token(TokenType::Select, 0..6).unwrap();
    feeder.feed_token(TokenType::Integer, 7..8).unwrap();

    // SEMI is shifted but the ecmd reduction hasn't fired yet (needs lookahead).
    let r = feeder.feed_token(TokenType::Semi, 8..9).unwrap();
    assert!(r.is_none());

    // finish() sends EOF which provides the lookahead.
    let root_id = feeder.finish().unwrap().expect("expected a statement");
    assert_eq!(feeder.node(root_id).unwrap().tag(), NodeTag::SelectStmt);
}

/// Multiple statements: the second statement's first token triggers
/// completion of the first statement.
#[test]
fn feed_tokens_multi_statement() {
    let source = "SELECT 1; SELECT 2";
    let mut tp = syntaqlite::low_level::TokenParser::new();
    let mut feeder = tp.feed(source);

    // First statement: SELECT 1 ;
    feeder.feed_token(TokenType::Select, 0..6).unwrap();
    feeder.feed_token(TokenType::Integer, 7..8).unwrap();
    let r = feeder.feed_token(TokenType::Semi, 8..9).unwrap();
    assert!(r.is_none()); // SEMI shifted, not reduced yet.

    // Second statement's first token provides the lookahead that completes stmt 1.
    let r = feeder.feed_token(TokenType::Select, 10..16).unwrap();
    let root1 = r.expect("first statement should complete on next SELECT");
    assert_eq!(feeder.node(root1).unwrap().tag(), NodeTag::SelectStmt);

    // Continue second statement.
    feeder.feed_token(TokenType::Integer, 17..18).unwrap();

    let root2 = feeder.finish().unwrap().expect("second statement");
    assert_eq!(feeder.node(root2).unwrap().tag(), NodeTag::SelectStmt);
}

/// TK_SPACE should be silently ignored.
#[test]
fn feed_token_skips_space() {
    let source = "SELECT 1";
    let mut tp = syntaqlite::low_level::TokenParser::new();
    let mut feeder = tp.feed(source);

    feeder.feed_token(TokenType::Select, 0..6).unwrap();

    // Feed a space — should be silently skipped.
    let r = feeder.feed_token(TokenType::Space, 6..7).unwrap();
    assert!(r.is_none());

    feeder.feed_token(TokenType::Integer, 7..8).unwrap();

    let root_id = feeder.finish().unwrap().expect("expected a statement");
    assert_eq!(feeder.node(root_id).unwrap().tag(), NodeTag::SelectStmt);
}

/// TK_COMMENT should be recorded as trivia.
#[test]
fn feed_token_records_comment_trivia() {
    // Source layout: "SELECT -- hello\n1"
    //                 0123456789...
    let source = "SELECT -- hello\n1";
    let mut tp = syntaqlite::low_level::TokenParser::new().with_collect_tokens();
    let mut feeder = tp.feed(source);

    feeder.feed_token(TokenType::Select, 0..6).unwrap();

    // Feed a line comment — "-- hello" starts at offset 7.
    feeder.feed_token(TokenType::Comment, 7..15).unwrap();

    feeder.feed_token(TokenType::Integer, 16..17).unwrap();

    let root_id = feeder.finish().unwrap().expect("expected a statement");
    assert_eq!(feeder.node(root_id).unwrap().tag(), NodeTag::SelectStmt);

    // Verify trivia was captured.
    let trivia = feeder.trivia();
    assert_eq!(trivia.len(), 1);
    assert_eq!(trivia[0].length, 8);
}

/// begin_macro / end_macro records macro regions.
#[test]
fn macro_regions_recorded() {
    let source = "SELECT 1";
    let mut tp = syntaqlite::low_level::TokenParser::new();
    let mut feeder = tp.feed(source);

    // Simulate a macro call at positions 7..20 in the original source.
    feeder.begin_macro(7, 13);

    feeder.feed_token(TokenType::Select, 0..6).unwrap();
    feeder.feed_token(TokenType::Integer, 7..8).unwrap();

    feeder.end_macro();

    feeder.finish().unwrap();

    let regions = feeder.macro_regions();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].call_offset, 7);
    assert_eq!(regions[0].call_length, 13);
}

/// Nested macro regions are both recorded.
#[test]
fn nested_macro_regions() {
    let source = "SELECT 1";
    let mut tp = syntaqlite::low_level::TokenParser::new();
    let mut feeder = tp.feed(source);

    feeder.begin_macro(0, 30);
    feeder.begin_macro(10, 5);

    feeder.feed_token(TokenType::Select, 0..6).unwrap();
    feeder.feed_token(TokenType::Integer, 7..8).unwrap();

    feeder.end_macro();
    feeder.end_macro();

    feeder.finish().unwrap();

    let regions = feeder.macro_regions();
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
    // Source: "SELECT foo!(1 + 2), 3"
    //          0123456789012345678901
    // Macro:         ^----------^  offset=7, length=11 → "foo!(1 + 2)"
    let source = "SELECT foo!(1 + 2), 3";
    let mut tp = syntaqlite::low_level::TokenParser::new();
    let mut feeder = tp.feed(source);

    feeder.feed_token(TokenType::Select, 0..6).unwrap();

    feeder.begin_macro(7, 11);
    // Expanded tokens: 1 + 2 (all spans inside macro region 7..18)
    feeder.feed_token(TokenType::Integer, 12..13).unwrap();
    feeder.feed_token(TokenType::Plus, 14..15).unwrap();
    feeder.feed_token(TokenType::Integer, 16..17).unwrap();
    feeder.end_macro();

    // Comma and "3" are outside the macro.
    feeder.feed_token(TokenType::Comma, 18..19).unwrap();
    feeder.feed_token(TokenType::Integer, 20..21).unwrap();

    let root = feeder.finish().unwrap().expect("expected a statement");
    assert_eq!(feeder.node(root).unwrap().tag(), NodeTag::SelectStmt);
}

/// A macro whose expanded tokens end up in a node that also contains
/// tokens from outside the macro region. The parser detects this straddle
/// and returns an error.
///
/// Source: "SELECT 1 FROM foo!(x) y"
///          0         1         2
///          01234567890123456789012345
/// Macro:                ^------^  offset=14, length=7 → "foo!(x)"
///
/// The macro expands to just the identifier "x". Then "y" is fed as a
/// regular token. SQLite treats "FROM x y" as "FROM x AS y", creating a
/// TableRef with table_name pointing at "x" (inside macro, offset 19)
/// and alias pointing at "y" (outside macro, offset 22). This straddles
/// the macro boundary — the parser rejects it.
#[test]
fn macro_straddle_rejected_by_parser() {
    let source = "SELECT 1 FROM foo!(x) y";
    let mut tp = syntaqlite::low_level::TokenParser::new();
    let mut feeder = tp.feed(source);

    feeder.feed_token(TokenType::Select, 0..6).unwrap();
    feeder.feed_token(TokenType::Integer, 7..8).unwrap();
    feeder.feed_token(TokenType::From, 9..13).unwrap();

    // Macro: "foo!(x)" at offset 14, length 7.
    feeder.begin_macro(14, 7);
    feeder.feed_token(TokenType::Id, 19..20).unwrap(); // "x"
    feeder.end_macro();

    // "y" outside macro — creates a straddling TableRef.
    feeder.feed_token(TokenType::Id, 22..23).unwrap();
    let err = feeder.finish().unwrap_err();
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
    let mut tp = syntaqlite::low_level::TokenParser::new();
    let mut feeder = tp.feed(source);

    let r = feeder.finish().unwrap();
    assert!(r.is_none());
}

/// High-level API still works after the refactor.
#[test]
fn high_level_api_still_works() {
    let mut parser = syntaqlite::Parser::new();
    let mut cursor = parser.parse("SELECT 1; SELECT 2");

    let r1 = cursor.next_statement().unwrap().unwrap();
    assert_eq!(cursor.node(r1).unwrap().tag(), NodeTag::SelectStmt);

    let r2 = cursor.next_statement().unwrap().unwrap();
    assert_eq!(cursor.node(r2).unwrap().tag(), NodeTag::SelectStmt);

    assert!(cursor.next_statement().is_none());
}

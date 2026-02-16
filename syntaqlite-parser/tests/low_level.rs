use syntaqlite_parser::{NodeTag, Parser, TokenType};

/// Feed tokens for "SELECT 1" via the low-level API and verify same AST
/// as the high-level parse.
#[test]
fn feed_tokens_select_1() {
    let source = "SELECT 1";
    let mut parser = Parser::new();
    let mut session = parser.parse(source);

    // Feed SELECT token.
    let r = session
        .feed_token(TokenType::Select as u32, &source[0..6])
        .unwrap();
    assert!(r.is_none());

    // Feed integer literal token.
    let r = session
        .feed_token(TokenType::Integer as u32, &source[7..8])
        .unwrap();
    assert!(r.is_none());

    // finish() synthesizes SEMI + EOF, triggering the ecmd reduction.
    let root_id = session.finish().unwrap().expect("expected a statement");
    let node = session.node(root_id).unwrap();
    assert_eq!(node.tag(), NodeTag::SelectStmt);
}

/// Feed tokens with an explicit SEMI. The LALR(1) parser needs one token of
/// lookahead after SEMI — the statement completes on finish() (which sends EOF).
#[test]
fn feed_tokens_with_semicolon() {
    let source = "SELECT 1;";
    let mut parser = Parser::new();
    let mut session = parser.parse(source);

    session
        .feed_token(TokenType::Select as u32, &source[0..6])
        .unwrap();
    session
        .feed_token(TokenType::Integer as u32, &source[7..8])
        .unwrap();

    // SEMI is shifted but the ecmd reduction hasn't fired yet (needs lookahead).
    let r = session
        .feed_token(TokenType::Semi as u32, &source[8..9])
        .unwrap();
    assert!(r.is_none());

    // finish() sends EOF which provides the lookahead.
    let root_id = session.finish().unwrap().expect("expected a statement");
    assert_eq!(session.node(root_id).unwrap().tag(), NodeTag::SelectStmt);
}

/// Multiple statements: the second statement's first token triggers
/// completion of the first statement.
#[test]
fn feed_tokens_multi_statement() {
    let source = "SELECT 1; SELECT 2";
    let mut parser = Parser::new();
    let mut session = parser.parse(source);

    // First statement: SELECT 1 ;
    session
        .feed_token(TokenType::Select as u32, &source[0..6])
        .unwrap();
    session
        .feed_token(TokenType::Integer as u32, &source[7..8])
        .unwrap();
    let r = session
        .feed_token(TokenType::Semi as u32, &source[8..9])
        .unwrap();
    assert!(r.is_none()); // SEMI shifted, not reduced yet.

    // Second statement's first token provides the lookahead that completes stmt 1.
    let r = session
        .feed_token(TokenType::Select as u32, &source[10..16])
        .unwrap();
    let root1 = r.expect("first statement should complete on next SELECT");
    assert_eq!(session.node(root1).unwrap().tag(), NodeTag::SelectStmt);

    // Continue second statement.
    session
        .feed_token(TokenType::Integer as u32, &source[17..18])
        .unwrap();

    let root2 = session.finish().unwrap().expect("second statement");
    assert_eq!(session.node(root2).unwrap().tag(), NodeTag::SelectStmt);
}

/// TK_SPACE should be silently ignored.
#[test]
fn feed_token_skips_space() {
    let source = "SELECT 1";
    let mut parser = Parser::new();
    let mut session = parser.parse(source);

    session
        .feed_token(TokenType::Select as u32, &source[0..6])
        .unwrap();

    // Feed a space — should be silently skipped.
    let r = session
        .feed_token(TokenType::Space as u32, &source[6..7])
        .unwrap();
    assert!(r.is_none());

    session
        .feed_token(TokenType::Integer as u32, &source[7..8])
        .unwrap();

    let root_id = session.finish().unwrap().expect("expected a statement");
    assert_eq!(session.node(root_id).unwrap().tag(), NodeTag::SelectStmt);
}

/// TK_COMMENT should be recorded as trivia.
#[test]
fn feed_token_records_comment_trivia() {
    // Source layout: "SELECT -- hello\n1"
    //                 0123456789...
    let source = "SELECT -- hello\n1";
    let mut parser = Parser::new();
    parser.set_collect_tokens(true);
    let mut session = parser.parse(source);

    session
        .feed_token(TokenType::Select as u32, &source[0..6])
        .unwrap();

    // Feed a line comment — "-- hello" starts at offset 7.
    session
        .feed_token(TokenType::Comment as u32, &source[7..15])
        .unwrap();

    session
        .feed_token(TokenType::Integer as u32, &source[16..17])
        .unwrap();

    let root_id = session.finish().unwrap().expect("expected a statement");
    assert_eq!(session.node(root_id).unwrap().tag(), NodeTag::SelectStmt);

    // Verify trivia was captured.
    let trivia = session.trivia();
    assert_eq!(trivia.len(), 1);
    assert_eq!(trivia[0].length, 8);
}

/// begin_macro / end_macro records macro regions.
#[test]
fn macro_regions_recorded() {
    let source = "SELECT 1";
    let mut parser = Parser::new();
    let mut session = parser.parse(source);

    // Simulate a macro call at positions 7..20 in the original source.
    session.begin_macro(7, 13);

    session
        .feed_token(TokenType::Select as u32, &source[0..6])
        .unwrap();
    session
        .feed_token(TokenType::Integer as u32, &source[7..8])
        .unwrap();

    session.end_macro();

    session.finish().unwrap();

    let regions = session.macro_regions();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].call_offset, 7);
    assert_eq!(regions[0].call_length, 13);
}

/// Nested macro regions are both recorded.
#[test]
fn nested_macro_regions() {
    let source = "SELECT 1";
    let mut parser = Parser::new();
    let mut session = parser.parse(source);

    session.begin_macro(0, 30);
    session.begin_macro(10, 5);

    session
        .feed_token(TokenType::Select as u32, &source[0..6])
        .unwrap();
    session
        .feed_token(TokenType::Integer as u32, &source[7..8])
        .unwrap();

    session.end_macro();
    session.end_macro();

    session.finish().unwrap();

    let regions = session.macro_regions();
    assert_eq!(regions.len(), 2);
    assert_eq!(regions[0].call_offset, 0);
    assert_eq!(regions[0].call_length, 30);
    assert_eq!(regions[1].call_offset, 10);
    assert_eq!(regions[1].call_length, 5);
}

/// finish() without feeding any tokens returns None.
#[test]
fn finish_with_no_tokens() {
    let source = "";
    let mut parser = Parser::new();
    let mut session = parser.parse(source);

    let r = session.finish().unwrap();
    assert!(r.is_none());
}

/// High-level API still works after the refactor.
#[test]
fn high_level_api_still_works() {
    let mut parser = Parser::new();
    let mut session = parser.parse("SELECT 1; SELECT 2");

    let r1 = session.next_statement().unwrap().unwrap();
    assert_eq!(session.node(r1).unwrap().tag(), NodeTag::SelectStmt);

    let r2 = session.next_statement().unwrap().unwrap();
    assert_eq!(session.node(r2).unwrap().tag(), NodeTag::SelectStmt);

    assert!(session.next_statement().is_none());
}

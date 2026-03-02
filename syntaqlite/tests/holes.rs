// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

/// Prototype tests: feeding TK_ILLEGAL inside macro regions to simulate
/// interpolation holes in embedded SQL (e.g. Python f-strings).
///
/// The idea: a host language scanner extracts SQL from strings like
///   f"SELECT * FROM {table} WHERE id = {user_id}"
/// and feeds tokens to the parser. For each `{...}` interpolation,
/// it calls begin_macro/end_macro around a TK_ILLEGAL token. The parser's
/// error recovery should create an ErrorNode, and the rest of the statement
/// should still parse correctly.
use syntaqlite::IncrementalParser;

mod tk {
    use syntaqlite::TokenType;
    pub const SELECT: TokenType = TokenType::SELECT;
    pub const STAR: TokenType = TokenType::STAR;
    pub const FROM: TokenType = TokenType::FROM;
    pub const WHERE: TokenType = TokenType::WHERE;
    pub const ID: TokenType = TokenType::ID;
    pub const EQ: TokenType = TokenType::EQ;
    pub const INTEGER: TokenType = TokenType::INTEGER;
    pub const ILLEGAL: TokenType = TokenType::ILLEGAL;
}

// ---------------------------------------------------------------------------
// Test 1: Hole in expression position (value)
// Source: f"SELECT * FROM users WHERE id = {user_id}"
//
// Expected: parser should build a SELECT with WHERE clause,
// the {user_id} hole creates an ErrorNode in expr position.
// ---------------------------------------------------------------------------
#[test]
fn hole_in_expr_position() {
    //        0123456789...
    let source = "SELECT * FROM users WHERE id = {user_id}";

    let mut tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    // SELECT
    cursor.feed_token(tk::SELECT, 0..6).unwrap();
    // *
    cursor.feed_token(tk::STAR, 7..8).unwrap();
    // FROM
    cursor.feed_token(tk::FROM, 9..13).unwrap();
    // users
    cursor.feed_token(tk::ID, 14..19).unwrap();
    // WHERE
    cursor.feed_token(tk::WHERE, 20..25).unwrap();
    // id
    cursor.feed_token(tk::ID, 26..28).unwrap();
    // =
    cursor.feed_token(tk::EQ, 29..30).unwrap();

    // Hole: {user_id} at offset 31, length 9
    cursor.begin_macro(31, 9);
    let hole_result = cursor.feed_token(tk::ILLEGAL, 31..40);
    cursor.end_macro();

    // Check what happened — did we get an error? Did the parser keep going?
    eprintln!(
        "hole_in_expr_position: feed_token(ILLEGAL) returned {:?}",
        hole_result
    );

    let result = cursor.finish();
    eprintln!("hole_in_expr_position: finish() returned {:?}", result);

    // Ideally: Ok(Some(root)) — the statement parsed with an error node in
    // the expr position. If we get Err, the current grammar doesn't have
    // fine-grained error recovery and we need new rules.
}

// ---------------------------------------------------------------------------
// Test 2: Hole in table name position
// Source: f"SELECT * FROM {table}"
//
// Expected: parser should build a SELECT with the table name as an ErrorNode.
// ---------------------------------------------------------------------------
#[test]
fn hole_in_table_name_position() {
    let source = "SELECT * FROM {table}";

    let mut tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    cursor.feed_token(tk::SELECT, 0..6).unwrap();
    cursor.feed_token(tk::STAR, 7..8).unwrap();
    cursor.feed_token(tk::FROM, 9..13).unwrap();

    // Hole: {table} at offset 14, length 7
    cursor.begin_macro(14, 7);
    let hole_result = cursor.feed_token(tk::ILLEGAL, 14..21);
    cursor.end_macro();

    eprintln!(
        "hole_in_table_name: feed_token(ILLEGAL) returned {:?}",
        hole_result
    );

    let result = cursor.finish();
    eprintln!("hole_in_table_name: finish() returned {:?}", result);
}

// ---------------------------------------------------------------------------
// Test 3: Hole in table name, then valid WHERE clause after
// Source: f"SELECT * FROM {table} WHERE id = 1"
//
// This is the critical test: can the parser recover from the hole and
// continue parsing the WHERE clause?
// ---------------------------------------------------------------------------
#[test]
fn hole_in_table_name_with_trailing_clause() {
    let source = "SELECT * FROM {table} WHERE id = 1";

    let mut tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    cursor.feed_token(tk::SELECT, 0..6).unwrap();
    cursor.feed_token(tk::STAR, 7..8).unwrap();
    cursor.feed_token(tk::FROM, 9..13).unwrap();

    // Hole: {table} at offset 14, length 7
    cursor.begin_macro(14, 7);
    let hole_result = cursor.feed_token(tk::ILLEGAL, 14..21);
    cursor.end_macro();

    eprintln!(
        "hole_with_trailing: feed_token(ILLEGAL) returned {:?}",
        hole_result
    );

    // Try to keep feeding — does the parser accept more tokens?
    let where_result = cursor.feed_token(tk::WHERE, 22..27);
    eprintln!(
        "hole_with_trailing: feed_token(WHERE) returned {:?}",
        where_result
    );

    let id_result = cursor.feed_token(tk::ID, 28..30);
    eprintln!(
        "hole_with_trailing: feed_token(id) returned {:?}",
        id_result
    );

    let eq_result = cursor.feed_token(tk::EQ, 31..32);
    eprintln!(
        "hole_with_trailing: feed_token(EQ) returned {:?}",
        eq_result
    );

    let int_result = cursor.feed_token(tk::INTEGER, 33..34);
    eprintln!(
        "hole_with_trailing: feed_token(INTEGER) returned {:?}",
        int_result
    );

    let result = cursor.finish();
    eprintln!("hole_with_trailing: finish() returned {:?}", result);
}

// ---------------------------------------------------------------------------
// Test 4: Multiple holes in one statement
// Source: f"SELECT {cols} FROM {table} WHERE {col} = {val}"
// ---------------------------------------------------------------------------
#[test]
fn multiple_holes() {
    let source = "SELECT {cols} FROM {table} WHERE {col} = {val}";

    let mut tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    cursor.feed_token(tk::SELECT, 0..6).unwrap();

    // Hole 1: {cols} in select column position
    cursor.begin_macro(7, 6);
    let r1 = cursor.feed_token(tk::ILLEGAL, 7..13);
    cursor.end_macro();
    eprintln!("multiple_holes: hole 1 (cols) = {:?}", r1);

    cursor.feed_token(tk::FROM, 14..18).unwrap();

    // Hole 2: {table} in table name position
    cursor.begin_macro(19, 7);
    let r2 = cursor.feed_token(tk::ILLEGAL, 19..26);
    cursor.end_macro();
    eprintln!("multiple_holes: hole 2 (table) = {:?}", r2);

    cursor.feed_token(tk::WHERE, 27..32).unwrap();

    // Hole 3: {col} in column ref position
    cursor.begin_macro(33, 5);
    let r3 = cursor.feed_token(tk::ILLEGAL, 33..38);
    cursor.end_macro();
    eprintln!("multiple_holes: hole 3 (col) = {:?}", r3);

    cursor.feed_token(tk::EQ, 39..40).unwrap();

    // Hole 4: {val} in expr position
    cursor.begin_macro(41, 5);
    let r4 = cursor.feed_token(tk::ILLEGAL, 41..46);
    cursor.end_macro();
    eprintln!("multiple_holes: hole 4 (val) = {:?}", r4);

    let result = cursor.finish();
    eprintln!("multiple_holes: finish() = {:?}", result);
}

// ---------------------------------------------------------------------------
// Test 5: Hole as entire trailing clause (case 2 — hardest)
// Source: f"SELECT * FROM users {extra}"
// ---------------------------------------------------------------------------
#[test]
fn hole_as_trailing_clause() {
    let source = "SELECT * FROM users {extra}";

    let mut tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    cursor.feed_token(tk::SELECT, 0..6).unwrap();
    cursor.feed_token(tk::STAR, 7..8).unwrap();
    cursor.feed_token(tk::FROM, 9..13).unwrap();
    cursor.feed_token(tk::ID, 14..19).unwrap();

    // Hole: {extra} — could be WHERE, ORDER BY, or anything
    cursor.begin_macro(20, 7);
    let hole_result = cursor.feed_token(tk::ILLEGAL, 20..27);
    cursor.end_macro();
    eprintln!(
        "trailing_clause: feed_token(ILLEGAL) returned {:?}",
        hole_result
    );

    let result = cursor.finish();
    eprintln!("trailing_clause: finish() returned {:?}", result);
}

// ---------------------------------------------------------------------------
// Baseline: Feeding TK_ID inside a macro region (current approach that works)
// Source: f"SELECT * FROM {table} WHERE id = 1"
// ---------------------------------------------------------------------------
#[test]
fn baseline_id_in_macro_region() {
    let source = "SELECT * FROM {table} WHERE id = 1";

    let mut tp = IncrementalParser::new();
    let mut cursor = tp.feed(source);

    cursor.feed_token(tk::SELECT, 0..6).unwrap();
    cursor.feed_token(tk::STAR, 7..8).unwrap();
    cursor.feed_token(tk::FROM, 9..13).unwrap();

    // Feed TK_ID instead of TK_ILLEGAL — this should always work
    cursor.begin_macro(14, 7);
    cursor.feed_token(tk::ID, 14..21).unwrap();
    cursor.end_macro();

    cursor.feed_token(tk::WHERE, 22..27).unwrap();
    cursor.feed_token(tk::ID, 28..30).unwrap();
    cursor.feed_token(tk::EQ, 31..32).unwrap();
    cursor.feed_token(tk::INTEGER, 33..34).unwrap();

    let root = cursor.finish().unwrap().expect("expected a statement");
    eprintln!("baseline: got root node {:?}", root);

    // Format it to see the macro region preserved
    let fmt = syntaqlite::Formatter::new();
    let formatted = fmt.format_node(cursor.root().unwrap());
    eprintln!("baseline formatted: {}", formatted);

    assert_eq!(formatted, "SELECT * FROM {table} WHERE id = 1");
}

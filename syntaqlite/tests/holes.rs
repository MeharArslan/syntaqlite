// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Prototype tests: feeding `TK_ILLEGAL` inside macro regions to simulate
//! interpolation holes in embedded SQL (e.g. Python f-strings).
//!
//! The idea: a host language scanner extracts SQL from strings like
//!   f"SELECT * FROM {table} WHERE id = {`user_id`}"
//! and feeds tokens to the parser. For each `{...}` interpolation,
//! it calls `begin_macro/end_macro` around a `TK_ILLEGAL` token. The parser's
//! error recovery should create an `ErrorNode`, and the rest of the statement
//! should still parse correctly.
use syntaqlite::Parser;

mod tk {
    use syntaqlite::TokenType;
    pub(crate) const SELECT: TokenType = TokenType::Select;
    pub(crate) const STAR: TokenType = TokenType::Star;
    pub(crate) const FROM: TokenType = TokenType::From;
    pub(crate) const WHERE: TokenType = TokenType::Where;
    pub(crate) const ID: TokenType = TokenType::Id;
    pub(crate) const EQ: TokenType = TokenType::Eq;
    pub(crate) const INTEGER: TokenType = TokenType::Integer;
    pub(crate) const ILLEGAL: TokenType = TokenType::Illegal;
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

    let parser = Parser::new();
    let mut cursor = parser.incremental_parse(source);

    // SELECT
    cursor.feed_token(tk::SELECT, 0..6);
    // *
    cursor.feed_token(tk::STAR, 7..8);
    // FROM
    cursor.feed_token(tk::FROM, 9..13);
    // users
    cursor.feed_token(tk::ID, 14..19);
    // WHERE
    cursor.feed_token(tk::WHERE, 20..25);
    // id
    cursor.feed_token(tk::ID, 26..28);
    // =
    cursor.feed_token(tk::EQ, 29..30);

    // Hole: {user_id} at offset 31, length 9
    cursor.begin_macro(31..31 + 9);
    let hole_result = cursor.feed_token(tk::ILLEGAL, 31..40);
    // Check what happened — did we get an error? Did the parser keep going?
    eprintln!(
        "hole_in_expr_position: feed_token(ILLEGAL) returned is_some={}",
        hole_result.is_some()
    );
    cursor.end_macro();

    let result = cursor.finish();
    eprintln!(
        "hole_in_expr_position: finish() returned is_some={}",
        result.is_some()
    );

    // Ideally: Some(Ok(..)) — the statement parsed with an error node in
    // the expr position. If we get Some(Err(..)), the current grammar doesn't
    // have fine-grained error recovery and we need new rules.
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

    let parser = Parser::new();
    let mut cursor = parser.incremental_parse(source);

    cursor.feed_token(tk::SELECT, 0..6);
    cursor.feed_token(tk::STAR, 7..8);
    cursor.feed_token(tk::FROM, 9..13);

    // Hole: {table} at offset 14, length 7
    cursor.begin_macro(14..14 + 7);
    let hole_result = cursor.feed_token(tk::ILLEGAL, 14..21);
    eprintln!(
        "hole_in_table_name: feed_token(ILLEGAL) returned is_some={}",
        hole_result.is_some()
    );
    cursor.end_macro();

    let result = cursor.finish();
    eprintln!(
        "hole_in_table_name: finish() returned is_some={}",
        result.is_some()
    );
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

    let parser = Parser::new();
    let mut cursor = parser.incremental_parse(source);

    cursor.feed_token(tk::SELECT, 0..6);
    cursor.feed_token(tk::STAR, 7..8);
    cursor.feed_token(tk::FROM, 9..13);

    // Hole: {table} at offset 14, length 7
    cursor.begin_macro(14..14 + 7);
    let hole_result = cursor.feed_token(tk::ILLEGAL, 14..21);
    eprintln!(
        "hole_with_trailing: feed_token(ILLEGAL) returned is_some={}",
        hole_result.is_some()
    );
    cursor.end_macro();

    // Try to keep feeding — does the parser accept more tokens?
    eprintln!(
        "hole_with_trailing: feed_token(WHERE) returned is_some={}",
        cursor.feed_token(tk::WHERE, 22..27).is_some()
    );
    eprintln!(
        "hole_with_trailing: feed_token(id) returned is_some={}",
        cursor.feed_token(tk::ID, 28..30).is_some()
    );
    eprintln!(
        "hole_with_trailing: feed_token(EQ) returned is_some={}",
        cursor.feed_token(tk::EQ, 31..32).is_some()
    );
    eprintln!(
        "hole_with_trailing: feed_token(INTEGER) returned is_some={}",
        cursor.feed_token(tk::INTEGER, 33..34).is_some()
    );

    let result = cursor.finish();
    eprintln!(
        "hole_with_trailing: finish() returned is_some={}",
        result.is_some()
    );
}

// ---------------------------------------------------------------------------
// Test 4: Multiple holes in one statement
// Source: f"SELECT {cols} FROM {table} WHERE {col} = {val}"
// ---------------------------------------------------------------------------
#[test]
fn multiple_holes() {
    let source = "SELECT {cols} FROM {table} WHERE {col} = {val}";

    let parser = Parser::new();
    let mut cursor = parser.incremental_parse(source);

    cursor.feed_token(tk::SELECT, 0..6);

    // Hole 1: {cols} in select column position
    cursor.begin_macro(7..7 + 6);
    eprintln!(
        "multiple_holes: hole 1 (cols) = is_some={}",
        cursor.feed_token(tk::ILLEGAL, 7..13).is_some()
    );
    cursor.end_macro();

    cursor.feed_token(tk::FROM, 14..18);

    // Hole 2: {table} in table name position
    cursor.begin_macro(19..19 + 7);
    eprintln!(
        "multiple_holes: hole 2 (table) = is_some={}",
        cursor.feed_token(tk::ILLEGAL, 19..26).is_some()
    );
    cursor.end_macro();

    cursor.feed_token(tk::WHERE, 27..32);

    // Hole 3: {col} in column ref position
    cursor.begin_macro(33..33 + 5);
    eprintln!(
        "multiple_holes: hole 3 (col) = is_some={}",
        cursor.feed_token(tk::ILLEGAL, 33..38).is_some()
    );
    cursor.end_macro();

    cursor.feed_token(tk::EQ, 39..40);

    // Hole 4: {val} in expr position
    cursor.begin_macro(41..41 + 5);
    eprintln!(
        "multiple_holes: hole 4 (val) = is_some={}",
        cursor.feed_token(tk::ILLEGAL, 41..46).is_some()
    );
    cursor.end_macro();

    let result = cursor.finish();
    eprintln!("multiple_holes: finish() = is_some={}", result.is_some());
}

// ---------------------------------------------------------------------------
// Test 5: Hole as entire trailing clause (case 2 — hardest)
// Source: f"SELECT * FROM users {extra}"
// ---------------------------------------------------------------------------
#[test]
fn hole_as_trailing_clause() {
    let source = "SELECT * FROM users {extra}";

    let parser = Parser::new();
    let mut cursor = parser.incremental_parse(source);

    cursor.feed_token(tk::SELECT, 0..6);
    cursor.feed_token(tk::STAR, 7..8);
    cursor.feed_token(tk::FROM, 9..13);
    cursor.feed_token(tk::ID, 14..19);

    // Hole: {extra} — could be WHERE, ORDER BY, or anything
    cursor.begin_macro(20..20 + 7);
    eprintln!(
        "trailing_clause: feed_token(ILLEGAL) returned is_some={}",
        cursor.feed_token(tk::ILLEGAL, 20..27).is_some()
    );
    cursor.end_macro();

    let result = cursor.finish();
    eprintln!(
        "trailing_clause: finish() returned is_some={}",
        result.is_some()
    );
}

// ---------------------------------------------------------------------------
// Baseline: Feeding TK_ID inside a macro region (current approach that works)
// Source: f"SELECT * FROM {table} WHERE id = 1"
// ---------------------------------------------------------------------------
#[test]
fn baseline_id_in_macro_region() {
    let source = "SELECT * FROM {table} WHERE id = 1";

    let parser = Parser::new();
    let mut cursor = parser.incremental_parse(source);

    cursor.feed_token(tk::SELECT, 0..6);
    cursor.feed_token(tk::STAR, 7..8);
    cursor.feed_token(tk::FROM, 9..13);

    // Feed TK_ID instead of TK_ILLEGAL — this should always work
    cursor.begin_macro(14..14 + 7);
    cursor.feed_token(tk::ID, 14..21);
    cursor.end_macro();

    cursor.feed_token(tk::WHERE, 22..27);
    cursor.feed_token(tk::ID, 28..30);
    cursor.feed_token(tk::EQ, 31..32);
    cursor.feed_token(tk::INTEGER, 33..34);

    let stmt = cursor
        .finish()
        .expect("expected Some")
        .expect("expected a statement");
    eprintln!("baseline: got root node {:?}", stmt.root());

    // Format it to see the macro region preserved
    let mut fmt = syntaqlite::Formatter::new();
    let formatted = fmt.format_parsed(stmt.erase());
    eprintln!("baseline formatted: {formatted}");

    assert_eq!(formatted, "SELECT * FROM {table} WHERE id = 1");
}

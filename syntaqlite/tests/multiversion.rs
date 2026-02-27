// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Oracle tests for version-gated tokenization and parsing.
//!
//! These tests verify that syntaqlite's tokenizer produces the same behavior as
//! real SQLite at version boundaries. The expected behavior was verified against
//! actual sqlite3 shells compiled from official amalgamations:
//!
//! - 3.37.2: `SELECT 1->2` → error near ">" (no -> operator)
//! - 3.38.0: `SELECT 1->2` → success (-> operator added)
//! - 3.45.3: `SELECT 1_000` → error "unrecognized token: 1_000"
//! - 3.46.0: `SELECT 1_000` → success (digit separators added)
//! - 3.34.1: `INSERT ... RETURNING` → error near "RETURNING"
//! - 3.35.0: same query → success
//! - 3.34.1: `WITH t AS MATERIALIZED (...)` → error near "MATERIALIZED"
//! - 3.35.0: same query → success
//! - 3.24.0: `SELECT sum(1) OVER ()` → error near "("
//! - 3.25.0: same query → success
//! - 3.23.1: `ON CONFLICT(x) DO NOTHING` → error near "ON"
//! - 3.24.0: same query → success
//! - 3.24.0: `FILTER (WHERE ...)` → error near "("
//! - 3.25.0: same query (with OVER) → success
//! - 3.47.0: WITHIN keyword added to mkkeywordhash (cflag-gated)
//! - 3.47.0 without ENABLE_ORDERED_SET_AGGREGATES: WITHIN treated as ID
//! - 3.47.0 with ENABLE_ORDERED_SET_AGGREGATES: WITHIN recognized as keyword

use syntaqlite::low_level::TokenType;
use syntaqlite_runtime::dialect::ffi::DialectConfig;

/// Shorthand: convert a TokenType variant to its raw u32 value.
const fn tk(t: TokenType) -> u32 {
    t as u32
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Tokenize SQL with a specific SQLite version and return (token_type, text) pairs,
/// filtering out whitespace.
fn tokenize_at_version(sql: &str, version: i32) -> Vec<(u32, String)> {
    let dialect = syntaqlite::low_level::dialect();
    let mut tok = syntaqlite_runtime::parser::Tokenizer::new(*dialect);
    tok.set_dialect_config(&DialectConfig {
        sqlite_version: version,
        ..Default::default()
    });
    tok.tokenize(sql)
        .filter(|raw| raw.token_type != tk(TokenType::Space))
        .map(|raw| (raw.token_type, raw.text.to_string()))
        .collect()
}

/// Tokenize SQL with latest version (default config).
fn tokenize_latest(sql: &str) -> Vec<(u32, String)> {
    tokenize_at_version(sql, i32::MAX)
}

/// Tokenize SQL with a specific version and cflag indices set.
fn tokenize_at_version_cflags(
    sql: &str,
    version: i32,
    cflag_indices: &[u32],
) -> Vec<(u32, String)> {
    let dialect = syntaqlite::low_level::dialect();
    let mut tok = syntaqlite_runtime::parser::Tokenizer::new(*dialect);
    let mut config = DialectConfig {
        sqlite_version: version,
        ..Default::default()
    };
    for &idx in cflag_indices {
        config.cflags.set(idx);
    }
    tok.set_dialect_config(&config);
    tok.tokenize(sql)
        .filter(|raw| raw.token_type != tk(TokenType::Space))
        .map(|raw| (raw.token_type, raw.text.to_string()))
        .collect()
}

/// Parse SQL with a specific SQLite version and return whether it succeeded.
fn parses_ok_at_version(sql: &str, version: i32) -> bool {
    parses_ok_at_version_cflags(sql, version, &[])
}

/// Parse SQL with a specific version and cflag indices set.
fn parses_ok_at_version_cflags(sql: &str, version: i32, cflag_indices: &[u32]) -> bool {
    let dialect = syntaqlite::low_level::dialect();
    let mut parser = syntaqlite_runtime::Parser::new(dialect);
    let mut config = DialectConfig {
        sqlite_version: version,
        ..Default::default()
    };
    for &idx in cflag_indices {
        config.cflags.set(idx);
    }
    parser.set_dialect_config(&config);
    let mut cursor = parser.parse(sql);
    matches!(cursor.next_statement(), Some(Ok(_)))
}

/// Helper to encode a SQLite version like 3.38.0 as the integer 3038000.
const fn ver(major: i32, minor: i32, patch: i32) -> i32 {
    major * 1_000_000 + minor * 1_000 + patch
}

// ---------------------------------------------------------------------------
// TK_PTR reclassification (-> and ->> operators, added in 3.38.0)
//
// Verified against real sqlite3 shells:
//   3.37.2: `SELECT '{"a":1}' -> '$.a';` → Error: near ">": syntax error
//   3.38.0: `SELECT '{"a":1}' -> '$.a';` → 1
// ---------------------------------------------------------------------------

#[test]
fn ptr_operator_tokenizes_as_ptr_on_latest() {
    let tokens = tokenize_latest("1->2");
    let types: Vec<_> = tokens.iter().map(|(tt, _)| *tt).collect();
    assert_eq!(
        types,
        vec![
            tk(TokenType::Integer),
            tk(TokenType::Ptr),
            tk(TokenType::Integer)
        ]
    );
}

#[test]
fn ptr_operator_reclassified_to_minus_before_3_38() {
    // Before 3.38, -> should tokenize as TK_MINUS (length 1), then '>' is a
    // separate GT token, then '2' is an integer.
    let tokens = tokenize_at_version("1->2", ver(3, 37, 0));
    let types: Vec<_> = tokens.iter().map(|(tt, _)| *tt).collect();
    assert_eq!(
        types,
        vec![
            tk(TokenType::Integer),
            tk(TokenType::Minus),
            tk(TokenType::Gt),
            tk(TokenType::Integer)
        ],
        "Before 3.38, '->' should split into MINUS + GT"
    );
    // Verify the minus token is just '-' (length 1).
    assert_eq!(tokens[1].1, "-");
}

#[test]
fn ptr_operator_works_at_3_38() {
    let tokens = tokenize_at_version("1->2", ver(3, 38, 0));
    let types: Vec<_> = tokens.iter().map(|(tt, _)| *tt).collect();
    assert_eq!(
        types,
        vec![
            tk(TokenType::Integer),
            tk(TokenType::Ptr),
            tk(TokenType::Integer)
        ]
    );
}

#[test]
fn double_ptr_reclassified_before_3_38() {
    // ->> should also split into MINUS + >> before 3.38.
    let tokens = tokenize_at_version("1->>2", ver(3, 37, 0));
    let types: Vec<_> = tokens.iter().map(|(tt, _)| *tt).collect();
    assert_eq!(
        types,
        vec![
            tk(TokenType::Integer),
            tk(TokenType::Minus),
            tk(TokenType::Rshift),
            tk(TokenType::Integer)
        ],
        "Before 3.38, '->>' should split into MINUS + RSHIFT"
    );
    assert_eq!(tokens[1].1, "-");
}

#[test]
fn ptr_reclassification_parse_fails_before_3_38() {
    // Verified: real SQLite 3.37.2 returns "near '>': syntax error"
    assert!(
        !parses_ok_at_version("SELECT 1->2;", ver(3, 37, 0)),
        "SELECT 1->2 should fail to parse before 3.38"
    );
}

#[test]
fn ptr_reclassification_parse_succeeds_at_3_38() {
    assert!(
        parses_ok_at_version("SELECT 1->2;", ver(3, 38, 0)),
        "SELECT 1->2 should parse at 3.38+"
    );
}

// ---------------------------------------------------------------------------
// TK_QNUMBER reclassification (digit separators, added in 3.46.0)
//
// Verified against real sqlite3 shells:
//   3.45.3: `SELECT 1_000;` → "unrecognized token: 1_000"
//   3.46.0: `SELECT 1_000;` → 1000
// ---------------------------------------------------------------------------

#[test]
fn digit_separator_tokenizes_as_qnumber_on_latest() {
    let tokens = tokenize_latest("1_000");
    let types: Vec<_> = tokens.iter().map(|(tt, _)| *tt).collect();
    assert_eq!(types, vec![tk(TokenType::Qnumber)]);
    assert_eq!(tokens[0].1, "1_000");
}

#[test]
fn digit_separator_reclassified_to_integer_before_3_46() {
    // Before 3.46, 1_000 should truncate to just "1" (INTEGER).
    let tokens = tokenize_at_version("1_000", ver(3, 45, 0));
    assert_eq!(
        tokens[0].0,
        tk(TokenType::Integer),
        "Should be INTEGER, not QNUMBER"
    );
    assert_eq!(
        tokens[0].1, "1",
        "Should truncate to '1' before the underscore"
    );
}

#[test]
fn digit_separator_float_reclassified_before_3_46() {
    // 1.5_0 should become FLOAT "1.5" before 3.46.
    let tokens = tokenize_at_version("1.5_0", ver(3, 45, 0));
    assert_eq!(
        tokens[0].0,
        tk(TokenType::Float),
        "Should be FLOAT, not QNUMBER"
    );
    assert_eq!(
        tokens[0].1, "1.5",
        "Should truncate to '1.5' before the underscore"
    );
}

#[test]
fn digit_separator_works_at_3_46() {
    let tokens = tokenize_at_version("1_000", ver(3, 46, 0));
    assert_eq!(tokens[0].0, tk(TokenType::Qnumber));
    assert_eq!(tokens[0].1, "1_000");
}

// ---------------------------------------------------------------------------
// Baseline: version-independent tokens should be unaffected
// ---------------------------------------------------------------------------

#[test]
fn basic_tokens_unaffected_by_version() {
    // These should tokenize identically regardless of version.
    for version in [ver(3, 12, 0), ver(3, 37, 0), ver(3, 46, 0), i32::MAX] {
        let tokens = tokenize_at_version("SELECT 1 + 2", version);
        let types: Vec<_> = tokens.iter().map(|(tt, _)| *tt).collect();
        assert_eq!(
            types,
            vec![
                tk(TokenType::Select),
                tk(TokenType::Integer),
                tk(TokenType::Plus),
                tk(TokenType::Integer)
            ],
            "Basic tokens should be stable at version {}",
            version
        );
    }
}

// ---------------------------------------------------------------------------
// Keyword version gating
//
// Verified against real sqlite3 shells compiled from official amalgamations:
//   3.34.1: `INSERT INTO t VALUES(1) RETURNING *;` → error near "RETURNING"
//   3.35.0: same query → success
//   3.34.1: `WITH t AS MATERIALIZED (SELECT 1) ...` → error near "MATERIALIZED"
//   3.35.0: same query → success
//   3.24.0: `SELECT sum(1) OVER ();` → error near "("
//   3.25.0: same query → success
//   3.23.1: `ON CONFLICT(x) DO NOTHING` → error near "ON"
//   3.24.0: same query → success
//   3.24.0: `FILTER (WHERE ...)` → error near "("
//   3.25.0: same query (with OVER) → success
//
// The tokenizer returns the keyword's fallback token (typically the same
// value the parser's %fallback would have produced) for versions older
// than the keyword's introduction. This prevents the parser from
// recognizing syntax that didn't exist at that version.
// ---------------------------------------------------------------------------

#[test]
fn returning_keyword_not_recognized_before_3_35() {
    // RETURNING was added as a keyword in 3.35.0.
    // Before that, it should NOT tokenize as TK_RETURNING.
    let tokens = tokenize_at_version("RETURNING", ver(3, 34, 0));
    assert_ne!(
        tokens[0].0,
        tk(TokenType::Returning),
        "RETURNING should not be a keyword before 3.35"
    );
}

#[test]
fn returning_keyword_recognized_at_3_35() {
    let tokens = tokenize_at_version("RETURNING", ver(3, 35, 0));
    assert_eq!(tokens[0].0, tk(TokenType::Returning));
}

#[test]
fn materialized_keyword_not_recognized_before_3_35() {
    let tokens = tokenize_at_version("MATERIALIZED", ver(3, 34, 0));
    assert_ne!(
        tokens[0].0,
        tk(TokenType::Materialized),
        "MATERIALIZED should not be a keyword before 3.35"
    );
}

#[test]
fn window_keyword_not_recognized_before_3_25() {
    // WINDOW was added in 3.25.0.
    let tokens = tokenize_at_version("WINDOW", ver(3, 24, 0));
    assert_ne!(
        tokens[0].0,
        tk(TokenType::Window),
        "WINDOW should not be a keyword before 3.25"
    );
}

#[test]
fn over_keyword_not_recognized_before_3_25() {
    let tokens = tokenize_at_version("OVER", ver(3, 24, 0));
    assert_ne!(
        tokens[0].0,
        tk(TokenType::Over),
        "OVER should not be a keyword before 3.25"
    );
}

#[test]
fn do_keyword_not_recognized_before_3_24() {
    // DO was added in 3.24.0 (upsert: ON CONFLICT DO).
    let tokens = tokenize_at_version("DO", ver(3, 23, 0));
    assert_ne!(
        tokens[0].0,
        tk(TokenType::Do),
        "DO should not be a keyword before 3.24"
    );
}

#[test]
fn filter_keyword_not_recognized_before_3_25() {
    // FILTER was added in 3.25.0 (with window functions).
    let tokens = tokenize_at_version("FILTER", ver(3, 24, 0));
    assert_ne!(
        tokens[0].0,
        tk(TokenType::Filter),
        "FILTER should not be a keyword before 3.25"
    );
}

// ---------------------------------------------------------------------------
// WITHIN keyword: cflag-gated (SQLITE_ENABLE_ORDERED_SET_AGGREGATES)
//
// WITHIN is an ENABLE-polarity keyword (polarity=1): it is only recognized
// when the SQLITE_ENABLE_ORDERED_SET_AGGREGATES flag IS set. Without the
// flag, WITHIN falls back to ID.
//
// Verified against SQLite 3.47.0+ compiled with/without the flag.
// ---------------------------------------------------------------------------

const CFLAG_ORDERED_SET: u32 = 36; // SYNQ_CFLAG_IDX_ENABLE_ORDERED_SET_AGGREGATES

#[test]
fn within_keyword_not_recognized_without_cflag() {
    // Without the ENABLE flag, WITHIN should NOT be recognized as TK_WITHIN.
    let tokens = tokenize_at_version("WITHIN", ver(3, 47, 0));
    assert_ne!(
        tokens[0].0,
        tk(TokenType::Within),
        "WITHIN should not be a keyword without ENABLE_ORDERED_SET_AGGREGATES"
    );
}

#[test]
fn within_keyword_recognized_with_cflag() {
    // With the ENABLE flag set, WITHIN should be recognized.
    let tokens = tokenize_at_version_cflags("WITHIN", ver(3, 47, 0), &[CFLAG_ORDERED_SET]);
    assert_eq!(
        tokens[0].0,
        tk(TokenType::Within),
        "WITHIN should be a keyword when ENABLE_ORDERED_SET_AGGREGATES is set"
    );
}

#[test]
fn within_keyword_not_recognized_before_3_47() {
    // Even with the cflag, WITHIN was not available before 3.47.
    let tokens = tokenize_at_version_cflags("WITHIN", ver(3, 46, 0), &[CFLAG_ORDERED_SET]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::Within),
        "WITHIN should not be a keyword before 3.47 even with cflag"
    );
}

#[test]
fn within_group_parses_with_cflag() {
    // percentile(0.5) WITHIN GROUP (ORDER BY salary)
    assert!(
        parses_ok_at_version_cflags(
            "SELECT percentile(0.5) WITHIN GROUP (ORDER BY salary) FROM t;",
            i32::MAX,
            &[CFLAG_ORDERED_SET],
        ),
        "WITHIN GROUP syntax should parse when cflag is set"
    );
}

#[test]
fn within_group_fails_without_cflag() {
    // Without the cflag, WITHIN falls back to ID and the syntax fails.
    assert!(
        !parses_ok_at_version_cflags(
            "SELECT percentile(0.5) WITHIN GROUP (ORDER BY salary) FROM t;",
            i32::MAX,
            &[],
        ),
        "WITHIN GROUP syntax should fail without cflag"
    );
}

// ---------------------------------------------------------------------------
// OMIT-polarity cflag oracle tests
//
// OMIT cflags suppress keywords at the tokenizer level, causing the keyword
// to fall back to ID. This prevents the parser from recognizing the gated
// syntax. Each test group verifies:
//   1. The keyword falls back to ID when the OMIT flag is set.
//   2. Parsing fails for a representative SQL statement.
// ---------------------------------------------------------------------------

const CFLAG_OMIT_WINDOWFUNC: u32 = 24; // SYNQ_CFLAG_IDX_OMIT_WINDOWFUNC
const CFLAG_OMIT_CTE: u32 = 7; // SYNQ_CFLAG_IDX_OMIT_CTE
const CFLAG_OMIT_RETURNING: u32 = 17; // SYNQ_CFLAG_IDX_OMIT_RETURNING
const CFLAG_OMIT_COMPOUND_SELECT: u32 = 6; // SYNQ_CFLAG_IDX_OMIT_COMPOUND_SELECT
// Note: OMIT_SUBQUERY (0x00000080) is not listed here because it doesn't gate
// any keywords — it uses the saw_subquery flag instead. See subquery tests below.
const CFLAG_OMIT_VIEW: u32 = 22; // SYNQ_CFLAG_IDX_OMIT_VIEW
const CFLAG_OMIT_TRIGGER: u32 = 20; // SYNQ_CFLAG_IDX_OMIT_TRIGGER

// ---- OMIT_WINDOWFUNC ----

#[test]
fn omit_windowfunc_keyword_falls_back_to_id() {
    let tokens = tokenize_at_version_cflags("OVER", i32::MAX, &[CFLAG_OMIT_WINDOWFUNC]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::Over),
        "OVER should fall back to ID with OMIT_WINDOWFUNC"
    );
}

#[test]
fn omit_windowfunc_parse_fails() {
    assert!(
        !parses_ok_at_version_cflags(
            "SELECT sum(x) OVER () FROM t;",
            i32::MAX,
            &[CFLAG_OMIT_WINDOWFUNC],
        ),
        "Window function syntax should fail with OMIT_WINDOWFUNC"
    );
}

// ---- OMIT_CTE ----

#[test]
fn omit_cte_keyword_falls_back_to_id() {
    // WITH is gated by OMIT_CTE.
    let tokens = tokenize_at_version_cflags("WITH", i32::MAX, &[CFLAG_OMIT_CTE]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::With),
        "WITH should fall back to ID with OMIT_CTE"
    );
}

#[test]
fn omit_cte_parse_fails() {
    assert!(
        !parses_ok_at_version_cflags(
            "WITH t AS (SELECT 1) SELECT * FROM t;",
            i32::MAX,
            &[CFLAG_OMIT_CTE],
        ),
        "CTE syntax should fail with OMIT_CTE"
    );
}

// ---- OMIT_RETURNING ----

#[test]
fn omit_returning_keyword_falls_back_to_id() {
    let tokens = tokenize_at_version_cflags("RETURNING", i32::MAX, &[CFLAG_OMIT_RETURNING]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::Returning),
        "RETURNING should fall back to ID with OMIT_RETURNING"
    );
}

#[test]
fn omit_returning_parse_fails() {
    assert!(
        !parses_ok_at_version_cflags(
            "INSERT INTO t VALUES(1) RETURNING *;",
            i32::MAX,
            &[CFLAG_OMIT_RETURNING],
        ),
        "RETURNING syntax should fail with OMIT_RETURNING"
    );
}

// ---- OMIT_COMPOUND_SELECT ----

#[test]
fn omit_compound_select_keyword_falls_back_to_id() {
    let tokens = tokenize_at_version_cflags("UNION", i32::MAX, &[CFLAG_OMIT_COMPOUND_SELECT]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::Union),
        "UNION should fall back to ID with OMIT_COMPOUND_SELECT"
    );
}

#[test]
fn omit_compound_select_parse_fails() {
    assert!(
        !parses_ok_at_version_cflags(
            "SELECT 1 UNION SELECT 2;",
            i32::MAX,
            &[CFLAG_OMIT_COMPOUND_SELECT],
        ),
        "UNION syntax should fail with OMIT_COMPOUND_SELECT"
    );
}

// ---- OMIT_VIEW ----

#[test]
fn omit_view_keyword_falls_back_to_id() {
    let tokens = tokenize_at_version_cflags("VIEW", i32::MAX, &[CFLAG_OMIT_VIEW]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::View),
        "VIEW should fall back to ID with OMIT_VIEW"
    );
}

#[test]
fn omit_view_parse_fails() {
    assert!(
        !parses_ok_at_version_cflags("CREATE VIEW v AS SELECT 1;", i32::MAX, &[CFLAG_OMIT_VIEW],),
        "CREATE VIEW syntax should fail with OMIT_VIEW"
    );
}

// ---- OMIT_TRIGGER ----

#[test]
fn omit_trigger_keyword_falls_back_to_id() {
    let tokens = tokenize_at_version_cflags("TRIGGER", i32::MAX, &[CFLAG_OMIT_TRIGGER]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::Trigger),
        "TRIGGER should fall back to ID with OMIT_TRIGGER"
    );
}

#[test]
fn omit_trigger_parse_fails() {
    assert!(
        !parses_ok_at_version_cflags(
            "CREATE TRIGGER t AFTER INSERT ON x BEGIN SELECT 1; END;",
            i32::MAX,
            &[CFLAG_OMIT_TRIGGER],
        ),
        "CREATE TRIGGER syntax should fail with OMIT_TRIGGER"
    );
}

// ---------------------------------------------------------------------------
// OMIT_SUBQUERY — uses saw_subquery flag, not keyword suppression
//
// SQLITE_OMIT_SUBQUERY gates grammar rules via %ifndef blocks, but all tokens
// used in subquery syntax (LP, SELECT, RP) are baseline tokens that can't be
// suppressed. Instead, grammar actions set saw_subquery on the parse context
// when a subquery production is reduced.
// ---------------------------------------------------------------------------

/// Parse SQL and return (success, saw_subquery).
fn parse_saw_subquery(sql: &str) -> (bool, bool) {
    let dialect = syntaqlite::low_level::dialect();
    let mut parser = syntaqlite_runtime::Parser::new(dialect);
    let mut cursor = parser.parse(sql);
    let ok = matches!(cursor.next_statement(), Some(Ok(_)));
    let saw = cursor.saw_subquery();
    (ok, saw)
}

#[test]
fn subquery_detected_in_from() {
    let (ok, saw) = parse_saw_subquery("SELECT * FROM (SELECT 1);");
    assert!(ok, "Should parse successfully");
    assert!(saw, "Should detect subquery in FROM clause");
}

#[test]
fn subquery_detected_in_exists() {
    let (ok, saw) = parse_saw_subquery("SELECT EXISTS (SELECT 1);");
    assert!(ok, "Should parse successfully");
    assert!(saw, "Should detect subquery in EXISTS expression");
}

#[test]
fn subquery_detected_in_scalar_subquery() {
    let (ok, saw) = parse_saw_subquery("SELECT (SELECT 1);");
    assert!(ok, "Should parse successfully");
    assert!(saw, "Should detect scalar subquery expression");
}

#[test]
fn subquery_detected_in_in_select() {
    let (ok, saw) = parse_saw_subquery("SELECT 1 WHERE 1 IN (SELECT 2);");
    assert!(ok, "Should parse successfully");
    assert!(saw, "Should detect subquery in IN (SELECT ...) expression");
}

#[test]
fn no_subquery_in_simple_select() {
    let (ok, saw) = parse_saw_subquery("SELECT 1;");
    assert!(ok, "Should parse successfully");
    assert!(!saw, "Simple SELECT should NOT set saw_subquery");
}

#[test]
fn no_subquery_in_in_list() {
    // IN with a literal list — not a subquery.
    let (ok, saw) = parse_saw_subquery("SELECT 1 WHERE 1 IN (1, 2, 3);");
    assert!(ok, "Should parse successfully");
    assert!(!saw, "IN with literal list should NOT set saw_subquery");
}

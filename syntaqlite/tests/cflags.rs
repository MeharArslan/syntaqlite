// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Oracle tests for cflag-gated keyword suppression and parsing.
//!
//! These tests verify that compile-time flags (cflags) correctly gate keywords
//! at the tokenizer level:
//!
//! - **OMIT-polarity** (polarity=0): keyword is suppressed (falls back to ID)
//!   when the OMIT flag IS set.
//! - **ENABLE-polarity** (polarity=1): keyword is only recognized when the
//!   ENABLE flag IS set; without it, falls back to ID.
//!
//! Each OMIT group tests:
//!   1. Keyword falls back to ID when the OMIT flag is set.
//!   2. Keyword is recognized normally without the flag.
//!   3. A representative SQL statement fails to parse with the flag.
//!   4. The same statement parses successfully without the flag.
//!
//! ENABLE groups additionally test version interaction (the keyword must
//! also meet its version requirement).
//!
//! The saw_subquery tests verify the OMIT_SUBQUERY detection mechanism,
//! which uses a parser flag rather than keyword suppression.

use syntaqlite::TokenType;
use syntaqlite::dialect::DialectConfig;

const fn tk(t: TokenType) -> u32 {
    t as u32
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Tokenize SQL with cflags set (latest version).
fn tokenize_with_cflags(sql: &str, cflag_indices: &[u32]) -> Vec<(u32, String)> {
    let dialect = syntaqlite::dialect::sqlite();
    let mut config = DialectConfig {
        sqlite_version: i32::MAX,
        ..Default::default()
    };
    for &idx in cflag_indices {
        config.cflags.set(idx);
    }
    let mut tok = syntaqlite::ext::RawTokenizer::builder(dialect)
        .dialect_config(config)
        .build();
    tok.tokenize(sql)
        .filter(|raw| raw.token_type != tk(TokenType::SPACE))
        .map(|raw| (raw.token_type, raw.text.to_string()))
        .collect()
}

/// Tokenize SQL with default config (no cflags, latest version).
fn tokenize_default(sql: &str) -> Vec<(u32, String)> {
    tokenize_with_cflags(sql, &[])
}

/// Tokenize SQL with a specific version and cflag indices set.
fn tokenize_at_version_cflags(
    sql: &str,
    version: i32,
    cflag_indices: &[u32],
) -> Vec<(u32, String)> {
    let dialect = syntaqlite::dialect::sqlite();
    let mut config = DialectConfig {
        sqlite_version: version,
        ..Default::default()
    };
    for &idx in cflag_indices {
        config.cflags.set(idx);
    }
    let mut tok = syntaqlite::ext::RawTokenizer::builder(dialect)
        .dialect_config(config)
        .build();
    tok.tokenize(sql)
        .filter(|raw| raw.token_type != tk(TokenType::SPACE))
        .map(|raw| (raw.token_type, raw.text.to_string()))
        .collect()
}

/// Parse SQL with cflags set (latest version) and return whether it succeeded.
fn parses_ok_with_cflags(sql: &str, cflag_indices: &[u32]) -> bool {
    let dialect = syntaqlite::dialect::sqlite();
    let mut config = DialectConfig {
        sqlite_version: i32::MAX,
        ..Default::default()
    };
    for &idx in cflag_indices {
        config.cflags.set(idx);
    }
    let mut parser = syntaqlite::ext::RawParser::builder(dialect)
        .dialect_config(config)
        .build();
    let mut cursor = parser.parse(sql);
    matches!(cursor.next_statement(), Some(Ok(_)))
}

/// Parse SQL with default config (no cflags, latest version).
fn parses_ok_default(sql: &str) -> bool {
    parses_ok_with_cflags(sql, &[])
}

/// Helper to encode a SQLite version like 3.47.0 as the integer 3047000.
const fn ver(major: i32, minor: i32, patch: i32) -> i32 {
    major * 1_000_000 + minor * 1_000 + patch
}

// ---------------------------------------------------------------------------
// Cflag index constants
// ---------------------------------------------------------------------------

// OMIT flags:
const CFLAG_OMIT_ATTACH: u32 = 2;
const CFLAG_OMIT_AUTOINCREMENT: u32 = 3;
const CFLAG_OMIT_CAST: u32 = 4;
const CFLAG_OMIT_COMPOUND_SELECT: u32 = 6;
const CFLAG_OMIT_CTE: u32 = 7;
const CFLAG_OMIT_EXPLAIN: u32 = 9;
const CFLAG_OMIT_FOREIGN_KEY: u32 = 11;
const CFLAG_OMIT_GENERATED_COLUMNS: u32 = 12;
const CFLAG_OMIT_PRAGMA: u32 = 15;
const CFLAG_OMIT_REINDEX: u32 = 16;
const CFLAG_OMIT_RETURNING: u32 = 17;
const CFLAG_OMIT_TRIGGER: u32 = 20;
const CFLAG_OMIT_VIEW: u32 = 22;
const CFLAG_OMIT_VIRTUALTABLE: u32 = 23;
const CFLAG_OMIT_WINDOWFUNC: u32 = 24;

// ENABLE flags:
const CFLAG_ENABLE_ORDERED_SET_AGGREGATES: u32 = 36;

// ===========================================================================
// ENABLE-polarity cflag tests
// ===========================================================================

// ---------------------------------------------------------------------------
// ENABLE_ORDERED_SET_AGGREGATES — gates: WITHIN
//
// WITHIN is an ENABLE-polarity keyword (polarity=1): it is only recognized
// when the SQLITE_ENABLE_ORDERED_SET_AGGREGATES flag IS set. Without the
// flag, WITHIN falls back to ID.
//
// Verified against SQLite 3.47.0+ compiled with/without the flag.
// ---------------------------------------------------------------------------

#[test]
fn within_keyword_not_recognized_without_cflag() {
    // Without the ENABLE flag, WITHIN should NOT be recognized as TK_WITHIN.
    let tokens = tokenize_with_cflags("WITHIN", &[]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::WITHIN),
        "WITHIN should not be a keyword without ENABLE_ORDERED_SET_AGGREGATES"
    );
}

#[test]
fn within_keyword_recognized_with_cflag() {
    // With the ENABLE flag set, WITHIN should be recognized.
    let tokens = tokenize_with_cflags("WITHIN", &[CFLAG_ENABLE_ORDERED_SET_AGGREGATES]);
    assert_eq!(
        tokens[0].0,
        tk(TokenType::WITHIN),
        "WITHIN should be a keyword when ENABLE_ORDERED_SET_AGGREGATES is set"
    );
}

#[test]
fn within_keyword_not_recognized_before_3_47() {
    // Even with the cflag, WITHIN was not available before 3.47.
    let tokens = tokenize_at_version_cflags(
        "WITHIN",
        ver(3, 46, 0),
        &[CFLAG_ENABLE_ORDERED_SET_AGGREGATES],
    );
    assert_ne!(
        tokens[0].0,
        tk(TokenType::WITHIN),
        "WITHIN should not be a keyword before 3.47 even with cflag"
    );
}

#[test]
fn within_group_parses_with_cflag() {
    assert!(
        parses_ok_with_cflags(
            "SELECT percentile(0.5) WITHIN GROUP (ORDER BY salary) FROM t;",
            &[CFLAG_ENABLE_ORDERED_SET_AGGREGATES],
        ),
        "WITHIN GROUP syntax should parse when cflag is set"
    );
}

#[test]
fn within_group_fails_without_cflag() {
    // Without the cflag, WITHIN falls back to ID and the syntax fails.
    assert!(
        !parses_ok_default("SELECT percentile(0.5) WITHIN GROUP (ORDER BY salary) FROM t;",),
        "WITHIN GROUP syntax should fail without cflag"
    );
}

// ===========================================================================
// OMIT-polarity cflag tests
//
// OMIT cflags suppress keywords at the tokenizer level, causing the keyword
// to fall back to ID. This prevents the parser from recognizing the gated
// syntax.
// ===========================================================================

// ---------------------------------------------------------------------------
// OMIT_WINDOWFUNC — gates: OVER, FILTER, WINDOW, CURRENT, EXCLUDE,
//                          FOLLOWING, GROUPS, OTHERS, PARTITION,
//                          PRECEDING, RANGE, TIES, UNBOUNDED
// ---------------------------------------------------------------------------

#[test]
fn omit_windowfunc_keyword_falls_back_to_id() {
    let tokens = tokenize_with_cflags("OVER", &[CFLAG_OMIT_WINDOWFUNC]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::OVER),
        "OVER should fall back to ID with OMIT_WINDOWFUNC"
    );
}

#[test]
fn omit_windowfunc_parse_fails() {
    assert!(
        !parses_ok_with_cflags("SELECT sum(x) OVER () FROM t;", &[CFLAG_OMIT_WINDOWFUNC],),
        "Window function syntax should fail with OMIT_WINDOWFUNC"
    );
}

#[test]
fn omit_windowfunc_parse_succeeds_without_flag() {
    assert!(
        parses_ok_default("SELECT sum(x) OVER () FROM t;"),
        "Window function syntax should parse without OMIT_WINDOWFUNC"
    );
}

// ---------------------------------------------------------------------------
// OMIT_CTE — gates: WITH, MATERIALIZED, RECURSIVE
// ---------------------------------------------------------------------------

#[test]
fn omit_cte_keyword_falls_back_to_id() {
    let tokens = tokenize_with_cflags("WITH", &[CFLAG_OMIT_CTE]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::WITH),
        "WITH should fall back to ID with OMIT_CTE"
    );
}

#[test]
fn omit_cte_parse_fails() {
    assert!(
        !parses_ok_with_cflags("WITH t AS (SELECT 1) SELECT * FROM t;", &[CFLAG_OMIT_CTE],),
        "CTE syntax should fail with OMIT_CTE"
    );
}

#[test]
fn omit_cte_parse_succeeds_without_flag() {
    assert!(
        parses_ok_default("WITH t AS (SELECT 1) SELECT * FROM t;"),
        "CTE syntax should parse without OMIT_CTE"
    );
}

// ---------------------------------------------------------------------------
// OMIT_RETURNING — gates: RETURNING
// ---------------------------------------------------------------------------

#[test]
fn omit_returning_keyword_falls_back_to_id() {
    let tokens = tokenize_with_cflags("RETURNING", &[CFLAG_OMIT_RETURNING]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::RETURNING),
        "RETURNING should fall back to ID with OMIT_RETURNING"
    );
}

#[test]
fn omit_returning_parse_fails() {
    assert!(
        !parses_ok_with_cflags(
            "INSERT INTO t VALUES(1) RETURNING *;",
            &[CFLAG_OMIT_RETURNING],
        ),
        "RETURNING syntax should fail with OMIT_RETURNING"
    );
}

#[test]
fn omit_returning_parse_succeeds_without_flag() {
    assert!(
        parses_ok_default("INSERT INTO t VALUES(1) RETURNING *;"),
        "RETURNING syntax should parse without OMIT_RETURNING"
    );
}

// ---------------------------------------------------------------------------
// OMIT_COMPOUND_SELECT — gates: UNION, EXCEPT, INTERSECT
// ---------------------------------------------------------------------------

#[test]
fn omit_compound_select_keyword_falls_back_to_id() {
    let tokens = tokenize_with_cflags("UNION", &[CFLAG_OMIT_COMPOUND_SELECT]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::UNION),
        "UNION should fall back to ID with OMIT_COMPOUND_SELECT"
    );
}

#[test]
fn omit_compound_select_parse_fails() {
    assert!(
        !parses_ok_with_cflags("SELECT 1 UNION SELECT 2;", &[CFLAG_OMIT_COMPOUND_SELECT],),
        "UNION syntax should fail with OMIT_COMPOUND_SELECT"
    );
}

#[test]
fn omit_compound_select_parse_succeeds_without_flag() {
    assert!(
        parses_ok_default("SELECT 1 UNION SELECT 2;"),
        "UNION syntax should parse without OMIT_COMPOUND_SELECT"
    );
}

// ---------------------------------------------------------------------------
// OMIT_VIEW — gates: VIEW
// ---------------------------------------------------------------------------

#[test]
fn omit_view_keyword_falls_back_to_id() {
    let tokens = tokenize_with_cflags("VIEW", &[CFLAG_OMIT_VIEW]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::VIEW),
        "VIEW should fall back to ID with OMIT_VIEW"
    );
}

#[test]
fn omit_view_parse_fails() {
    assert!(
        !parses_ok_with_cflags("CREATE VIEW v AS SELECT 1;", &[CFLAG_OMIT_VIEW]),
        "CREATE VIEW syntax should fail with OMIT_VIEW"
    );
}

#[test]
fn omit_view_parse_succeeds_without_flag() {
    assert!(
        parses_ok_default("CREATE VIEW v AS SELECT 1;"),
        "CREATE VIEW syntax should parse without OMIT_VIEW"
    );
}

// ---------------------------------------------------------------------------
// OMIT_TRIGGER — gates: TRIGGER, AFTER, BEFORE, EACH, FOR, INSTEAD,
//                        RAISE, ROW
// ---------------------------------------------------------------------------

#[test]
fn omit_trigger_keyword_falls_back_to_id() {
    let tokens = tokenize_with_cflags("TRIGGER", &[CFLAG_OMIT_TRIGGER]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::TRIGGER),
        "TRIGGER should fall back to ID with OMIT_TRIGGER"
    );
}

#[test]
fn omit_trigger_parse_fails() {
    assert!(
        !parses_ok_with_cflags(
            "CREATE TRIGGER t AFTER INSERT ON x BEGIN SELECT 1; END;",
            &[CFLAG_OMIT_TRIGGER],
        ),
        "CREATE TRIGGER syntax should fail with OMIT_TRIGGER"
    );
}

#[test]
fn omit_trigger_parse_succeeds_without_flag() {
    assert!(
        parses_ok_default("CREATE TRIGGER t AFTER INSERT ON x BEGIN SELECT 1; END;",),
        "CREATE TRIGGER syntax should parse without OMIT_TRIGGER"
    );
}

// ---------------------------------------------------------------------------
// OMIT_FOREIGN_KEY — gates: ACTION, CASCADE, DEFERRABLE, FOREIGN,
//                           INITIALLY, REFERENCES, RESTRICT
// ---------------------------------------------------------------------------

#[test]
fn omit_foreign_key_keywords_fall_back_to_id() {
    let cflags = &[CFLAG_OMIT_FOREIGN_KEY];
    for (sql, expected_tt, name) in [
        ("FOREIGN", tk(TokenType::FOREIGN), "FOREIGN"),
        ("REFERENCES", tk(TokenType::REFERENCES), "REFERENCES"),
        ("CASCADE", tk(TokenType::CASCADE), "CASCADE"),
        ("RESTRICT", tk(TokenType::RESTRICT), "RESTRICT"),
        ("DEFERRABLE", tk(TokenType::DEFERRABLE), "DEFERRABLE"),
        ("INITIALLY", tk(TokenType::INITIALLY), "INITIALLY"),
        ("ACTION", tk(TokenType::ACTION), "ACTION"),
    ] {
        let tokens = tokenize_with_cflags(sql, cflags);
        assert_ne!(
            tokens[0].0, expected_tt,
            "{name} should fall back to ID with OMIT_FOREIGN_KEY"
        );
    }
}

#[test]
fn omit_foreign_key_keywords_recognized_without_flag() {
    for (sql, expected_tt, name) in [
        ("FOREIGN", tk(TokenType::FOREIGN), "FOREIGN"),
        ("REFERENCES", tk(TokenType::REFERENCES), "REFERENCES"),
    ] {
        let tokens = tokenize_default(sql);
        assert_eq!(
            tokens[0].0, expected_tt,
            "{name} should be recognized without OMIT_FOREIGN_KEY"
        );
    }
}

#[test]
fn omit_foreign_key_parse_fails() {
    assert!(
        !parses_ok_with_cflags(
            "CREATE TABLE t(x INTEGER REFERENCES other(id));",
            &[CFLAG_OMIT_FOREIGN_KEY],
        ),
        "REFERENCES syntax should fail with OMIT_FOREIGN_KEY"
    );
}

#[test]
fn omit_foreign_key_parse_succeeds_without_flag() {
    assert!(
        parses_ok_default("CREATE TABLE t(x INTEGER REFERENCES other(id));"),
        "REFERENCES syntax should parse without OMIT_FOREIGN_KEY"
    );
}

// ---------------------------------------------------------------------------
// OMIT_GENERATED_COLUMNS — gates: ALWAYS
// ---------------------------------------------------------------------------

#[test]
fn omit_generated_columns_keyword_falls_back_to_id() {
    let tokens = tokenize_with_cflags("ALWAYS", &[CFLAG_OMIT_GENERATED_COLUMNS]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::ALWAYS),
        "ALWAYS should fall back to ID with OMIT_GENERATED_COLUMNS"
    );
}

#[test]
fn omit_generated_columns_keyword_recognized_without_flag() {
    let tokens = tokenize_default("ALWAYS");
    assert_eq!(
        tokens[0].0,
        tk(TokenType::ALWAYS),
        "ALWAYS should be recognized without OMIT_GENERATED_COLUMNS"
    );
}

// ---------------------------------------------------------------------------
// OMIT_EXPLAIN — gates: EXPLAIN, PLAN, QUERY
// ---------------------------------------------------------------------------

#[test]
fn omit_explain_keywords_fall_back_to_id() {
    let cflags = &[CFLAG_OMIT_EXPLAIN];
    for (sql, expected_tt, name) in [
        ("EXPLAIN", tk(TokenType::EXPLAIN), "EXPLAIN"),
        ("QUERY", tk(TokenType::QUERY), "QUERY"),
    ] {
        let tokens = tokenize_with_cflags(sql, cflags);
        assert_ne!(
            tokens[0].0, expected_tt,
            "{name} should fall back to ID with OMIT_EXPLAIN"
        );
    }
}

#[test]
fn omit_explain_keywords_recognized_without_flag() {
    let tokens = tokenize_default("EXPLAIN");
    assert_eq!(
        tokens[0].0,
        tk(TokenType::EXPLAIN),
        "EXPLAIN should be recognized without OMIT_EXPLAIN"
    );
}

#[test]
fn omit_explain_parse_fails() {
    assert!(
        !parses_ok_with_cflags("EXPLAIN SELECT 1;", &[CFLAG_OMIT_EXPLAIN]),
        "EXPLAIN syntax should fail with OMIT_EXPLAIN"
    );
}

#[test]
fn omit_explain_parse_succeeds_without_flag() {
    assert!(
        parses_ok_default("EXPLAIN SELECT 1;"),
        "EXPLAIN syntax should parse without OMIT_EXPLAIN"
    );
}

#[test]
fn omit_explain_query_plan_parse_fails() {
    assert!(
        !parses_ok_with_cflags("EXPLAIN QUERY PLAN SELECT 1;", &[CFLAG_OMIT_EXPLAIN]),
        "EXPLAIN QUERY PLAN should fail with OMIT_EXPLAIN"
    );
}

#[test]
fn omit_explain_query_plan_parse_succeeds_without_flag() {
    assert!(
        parses_ok_default("EXPLAIN QUERY PLAN SELECT 1;"),
        "EXPLAIN QUERY PLAN should parse without OMIT_EXPLAIN"
    );
}

// ---------------------------------------------------------------------------
// OMIT_ATTACH — gates: ATTACH, DATABASE, DETACH
// ---------------------------------------------------------------------------

#[test]
fn omit_attach_keywords_fall_back_to_id() {
    let cflags = &[CFLAG_OMIT_ATTACH];
    for (sql, expected_tt, name) in [
        ("ATTACH", tk(TokenType::ATTACH), "ATTACH"),
        ("DATABASE", tk(TokenType::DATABASE), "DATABASE"),
        ("DETACH", tk(TokenType::DETACH), "DETACH"),
    ] {
        let tokens = tokenize_with_cflags(sql, cflags);
        assert_ne!(
            tokens[0].0, expected_tt,
            "{name} should fall back to ID with OMIT_ATTACH"
        );
    }
}

#[test]
fn omit_attach_keywords_recognized_without_flag() {
    for (sql, expected_tt, name) in [
        ("ATTACH", tk(TokenType::ATTACH), "ATTACH"),
        ("DETACH", tk(TokenType::DETACH), "DETACH"),
        ("DATABASE", tk(TokenType::DATABASE), "DATABASE"),
    ] {
        let tokens = tokenize_default(sql);
        assert_eq!(
            tokens[0].0, expected_tt,
            "{name} should be recognized without OMIT_ATTACH"
        );
    }
}

#[test]
fn omit_attach_parse_fails() {
    assert!(
        !parses_ok_with_cflags("ATTACH DATABASE ':memory:' AS db2;", &[CFLAG_OMIT_ATTACH],),
        "ATTACH DATABASE syntax should fail with OMIT_ATTACH"
    );
}

#[test]
fn omit_attach_parse_succeeds_without_flag() {
    assert!(
        parses_ok_default("ATTACH DATABASE ':memory:' AS db2;"),
        "ATTACH DATABASE syntax should parse without OMIT_ATTACH"
    );
}

#[test]
fn omit_detach_parse_fails() {
    assert!(
        !parses_ok_with_cflags("DETACH DATABASE db2;", &[CFLAG_OMIT_ATTACH]),
        "DETACH DATABASE syntax should fail with OMIT_ATTACH"
    );
}

#[test]
fn omit_detach_parse_succeeds_without_flag() {
    assert!(
        parses_ok_default("DETACH DATABASE db2;"),
        "DETACH DATABASE syntax should parse without OMIT_ATTACH"
    );
}

// ---------------------------------------------------------------------------
// OMIT_AUTOINCREMENT — gates: AUTOINCREMENT
// ---------------------------------------------------------------------------

#[test]
fn omit_autoincrement_keyword_falls_back_to_id() {
    let tokens = tokenize_with_cflags("AUTOINCREMENT", &[CFLAG_OMIT_AUTOINCREMENT]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::AUTOINCR),
        "AUTOINCREMENT should fall back to ID with OMIT_AUTOINCREMENT"
    );
}

#[test]
fn omit_autoincrement_keyword_recognized_without_flag() {
    let tokens = tokenize_default("AUTOINCREMENT");
    assert_eq!(
        tokens[0].0,
        tk(TokenType::AUTOINCR),
        "AUTOINCREMENT should be recognized without OMIT_AUTOINCREMENT"
    );
}

#[test]
fn omit_autoincrement_parse_fails() {
    assert!(
        !parses_ok_with_cflags(
            "CREATE TABLE t(id INTEGER PRIMARY KEY AUTOINCREMENT);",
            &[CFLAG_OMIT_AUTOINCREMENT],
        ),
        "AUTOINCREMENT syntax should fail with OMIT_AUTOINCREMENT"
    );
}

#[test]
fn omit_autoincrement_parse_succeeds_without_flag() {
    assert!(
        parses_ok_default("CREATE TABLE t(id INTEGER PRIMARY KEY AUTOINCREMENT);"),
        "AUTOINCREMENT syntax should parse without OMIT_AUTOINCREMENT"
    );
}

// ---------------------------------------------------------------------------
// OMIT_CAST — gates: CAST
// ---------------------------------------------------------------------------

#[test]
fn omit_cast_keyword_falls_back_to_id() {
    let tokens = tokenize_with_cflags("CAST", &[CFLAG_OMIT_CAST]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::CAST),
        "CAST should fall back to ID with OMIT_CAST"
    );
}

#[test]
fn omit_cast_keyword_recognized_without_flag() {
    let tokens = tokenize_default("CAST");
    assert_eq!(
        tokens[0].0,
        tk(TokenType::CAST),
        "CAST should be recognized without OMIT_CAST"
    );
}

#[test]
fn omit_cast_parse_fails() {
    assert!(
        !parses_ok_with_cflags("SELECT CAST(1 AS TEXT);", &[CFLAG_OMIT_CAST],),
        "CAST syntax should fail with OMIT_CAST"
    );
}

#[test]
fn omit_cast_parse_succeeds_without_flag() {
    assert!(
        parses_ok_default("SELECT CAST(1 AS TEXT);"),
        "CAST syntax should parse without OMIT_CAST"
    );
}

// ---------------------------------------------------------------------------
// OMIT_PRAGMA — gates: PRAGMA
// ---------------------------------------------------------------------------

#[test]
fn omit_pragma_keyword_falls_back_to_id() {
    let tokens = tokenize_with_cflags("PRAGMA", &[CFLAG_OMIT_PRAGMA]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::PRAGMA),
        "PRAGMA should fall back to ID with OMIT_PRAGMA"
    );
}

#[test]
fn omit_pragma_keyword_recognized_without_flag() {
    let tokens = tokenize_default("PRAGMA");
    assert_eq!(
        tokens[0].0,
        tk(TokenType::PRAGMA),
        "PRAGMA should be recognized without OMIT_PRAGMA"
    );
}

#[test]
fn omit_pragma_parse_fails() {
    assert!(
        !parses_ok_with_cflags("PRAGMA table_info('t');", &[CFLAG_OMIT_PRAGMA]),
        "PRAGMA syntax should fail with OMIT_PRAGMA"
    );
}

#[test]
fn omit_pragma_parse_succeeds_without_flag() {
    assert!(
        parses_ok_default("PRAGMA table_info('t');"),
        "PRAGMA syntax should parse without OMIT_PRAGMA"
    );
}

// ---------------------------------------------------------------------------
// OMIT_REINDEX — gates: REINDEX
// ---------------------------------------------------------------------------

#[test]
fn omit_reindex_keyword_falls_back_to_id() {
    let tokens = tokenize_with_cflags("REINDEX", &[CFLAG_OMIT_REINDEX]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::REINDEX),
        "REINDEX should fall back to ID with OMIT_REINDEX"
    );
}

#[test]
fn omit_reindex_keyword_recognized_without_flag() {
    let tokens = tokenize_default("REINDEX");
    assert_eq!(
        tokens[0].0,
        tk(TokenType::REINDEX),
        "REINDEX should be recognized without OMIT_REINDEX"
    );
}

#[test]
fn omit_reindex_parse_fails() {
    assert!(
        !parses_ok_with_cflags("REINDEX;", &[CFLAG_OMIT_REINDEX]),
        "REINDEX syntax should fail with OMIT_REINDEX"
    );
}

#[test]
fn omit_reindex_parse_succeeds_without_flag() {
    assert!(
        parses_ok_default("REINDEX;"),
        "REINDEX syntax should parse without OMIT_REINDEX"
    );
}

// ---------------------------------------------------------------------------
// OMIT_VIRTUALTABLE — gates: VIRTUAL
// ---------------------------------------------------------------------------

#[test]
fn omit_virtualtable_keyword_falls_back_to_id() {
    let tokens = tokenize_with_cflags("VIRTUAL", &[CFLAG_OMIT_VIRTUALTABLE]);
    assert_ne!(
        tokens[0].0,
        tk(TokenType::VIRTUAL),
        "VIRTUAL should fall back to ID with OMIT_VIRTUALTABLE"
    );
}

#[test]
fn omit_virtualtable_keyword_recognized_without_flag() {
    let tokens = tokenize_default("VIRTUAL");
    assert_eq!(
        tokens[0].0,
        tk(TokenType::VIRTUAL),
        "VIRTUAL should be recognized without OMIT_VIRTUALTABLE"
    );
}

#[test]
fn omit_virtualtable_parse_fails() {
    assert!(
        !parses_ok_with_cflags(
            "CREATE VIRTUAL TABLE t USING fts5(content);",
            &[CFLAG_OMIT_VIRTUALTABLE],
        ),
        "CREATE VIRTUAL TABLE syntax should fail with OMIT_VIRTUALTABLE"
    );
}

#[test]
fn omit_virtualtable_parse_succeeds_without_flag() {
    assert!(
        parses_ok_default("CREATE VIRTUAL TABLE t USING fts5(content);"),
        "CREATE VIRTUAL TABLE syntax should parse without OMIT_VIRTUALTABLE"
    );
}

// ===========================================================================
// Cross-cutting cflag tests
// ===========================================================================

// ---------------------------------------------------------------------------
// Multiple cflags combined
// ---------------------------------------------------------------------------

#[test]
fn multiple_omit_flags_suppress_independently() {
    let cflags = &[CFLAG_OMIT_EXPLAIN, CFLAG_OMIT_ATTACH];

    // EXPLAIN suppressed
    let tokens = tokenize_with_cflags("EXPLAIN", cflags);
    assert_ne!(tokens[0].0, tk(TokenType::EXPLAIN));

    // ATTACH suppressed
    let tokens = tokenize_with_cflags("ATTACH", cflags);
    assert_ne!(tokens[0].0, tk(TokenType::ATTACH));

    // SELECT still works (not gated by either flag)
    assert!(
        parses_ok_with_cflags("SELECT 1;", cflags),
        "SELECT should still parse with unrelated OMIT flags"
    );
}

#[test]
fn omit_flag_does_not_affect_unrelated_keywords() {
    // OMIT_CAST should not affect PRAGMA
    let tokens = tokenize_with_cflags("PRAGMA", &[CFLAG_OMIT_CAST]);
    assert_eq!(
        tokens[0].0,
        tk(TokenType::PRAGMA),
        "PRAGMA should not be affected by OMIT_CAST"
    );

    // OMIT_PRAGMA should not affect CAST
    let tokens = tokenize_with_cflags("CAST", &[CFLAG_OMIT_PRAGMA]);
    assert_eq!(
        tokens[0].0,
        tk(TokenType::CAST),
        "CAST should not be affected by OMIT_PRAGMA"
    );
}

// ===========================================================================
// ENABLE_UPDATE_DELETE_LIMIT — parser-level cflag check
//
// ORDER BY / LIMIT on DELETE and UPDATE require the
// SQLITE_ENABLE_UPDATE_DELETE_LIMIT cflag. The grammar actions check
// the cflag at parse time and raise a syntax error if it's not set.
// ===========================================================================

const CFLAG_ENABLE_UPDATE_DELETE_LIMIT: u32 = 40;

#[test]
fn delete_with_order_by_limit_fails_without_cflag() {
    assert!(
        !parses_ok_default("DELETE FROM t ORDER BY id LIMIT 5;"),
        "DELETE with ORDER BY/LIMIT should fail without ENABLE_UPDATE_DELETE_LIMIT"
    );
}

#[test]
fn delete_with_order_by_limit_succeeds_with_cflag() {
    assert!(
        parses_ok_with_cflags(
            "DELETE FROM t ORDER BY id LIMIT 5;",
            &[CFLAG_ENABLE_UPDATE_DELETE_LIMIT],
        ),
        "DELETE with ORDER BY/LIMIT should parse when ENABLE_UPDATE_DELETE_LIMIT is set"
    );
}

#[test]
fn update_with_limit_fails_without_cflag() {
    assert!(
        !parses_ok_default("UPDATE t SET a = 1 ORDER BY id LIMIT 3;"),
        "UPDATE with ORDER BY/LIMIT should fail without ENABLE_UPDATE_DELETE_LIMIT"
    );
}

#[test]
fn update_with_limit_succeeds_with_cflag() {
    assert!(
        parses_ok_with_cflags(
            "UPDATE t SET a = 1 ORDER BY id LIMIT 3;",
            &[CFLAG_ENABLE_UPDATE_DELETE_LIMIT],
        ),
        "UPDATE with ORDER BY/LIMIT should parse when ENABLE_UPDATE_DELETE_LIMIT is set"
    );
}

#[test]
fn plain_delete_succeeds_without_cflag() {
    assert!(
        parses_ok_default("DELETE FROM t WHERE x = 1;"),
        "Plain DELETE should parse without ENABLE_UPDATE_DELETE_LIMIT"
    );
}

#[test]
fn plain_update_succeeds_without_cflag() {
    assert!(
        parses_ok_default("UPDATE t SET a = 1 WHERE x = 2;"),
        "Plain UPDATE should parse without ENABLE_UPDATE_DELETE_LIMIT"
    );
}

#![cfg(all(feature = "fmt", feature = "sqlite"))]
//! Integration tests for public formatter keyword-case configuration.

use syntaqlite::{FormatConfig, Formatter, KeywordCase};

#[test]
fn keyword_case_upper() {
    let cfg = FormatConfig {
        keyword_case: KeywordCase::Upper,
        ..FormatConfig::default()
    };
    let mut fmt = Formatter::with_config(&cfg);
    let out = fmt
        .format("select 1")
        .expect("formatting should succeed for valid SQL");
    assert_eq!(out, "SELECT 1;\n");
}

#[test]
fn keyword_case_lower() {
    let cfg = FormatConfig {
        keyword_case: KeywordCase::Lower,
        ..FormatConfig::default()
    };
    let mut fmt = Formatter::with_config(&cfg);
    let out = fmt
        .format("SELECT 1")
        .expect("formatting should succeed for valid SQL");
    assert_eq!(out, "select 1;\n");
}

#[test]
fn keyword_case_preserve() {
    let cfg = FormatConfig {
        keyword_case: KeywordCase::Preserve,
        ..FormatConfig::default()
    };
    let mut fmt = Formatter::with_config(&cfg);
    let out = fmt
        .format("SeLeCt 1")
        .expect("formatting should succeed for valid SQL");
    assert_eq!(out, "SeLeCt 1;\n");
}

// --- preserve with multi-clause queries ---

#[test]
fn preserve_select_from() {
    let cfg = FormatConfig {
        keyword_case: KeywordCase::Preserve,
        ..FormatConfig::default()
    };
    let mut fmt = Formatter::with_config(&cfg);
    let out = fmt
        .format("SeLeCt id FrOm foo")
        .expect("formatting should succeed");
    assert_eq!(out, "SeLeCt id FrOm foo;\n");
}

#[test]
fn preserve_select_from_where() {
    let cfg = FormatConfig {
        keyword_case: KeywordCase::Preserve,
        ..FormatConfig::default()
    };
    let mut fmt = Formatter::with_config(&cfg);
    let out = fmt
        .format("SELECT id FROM foo WHERE x = 1")
        .expect("formatting should succeed");
    assert_eq!(out, "SELECT id FROM foo WHERE x = 1;\n");
}

#[test]
fn preserve_spaces_not_squashed_upper_kw() {
    // Ensure that when keyword_case=Preserve and keywords happen to be uppercase,
    // spaces around identifiers and between clauses are preserved correctly.
    let cfg = FormatConfig {
        keyword_case: KeywordCase::Preserve,
        ..FormatConfig::default()
    };
    let mut fmt = Formatter::with_config(&cfg);
    let out = fmt
        .format("SELECT a, b FROM t")
        .expect("formatting should succeed");
    assert_eq!(out, "SELECT a, b FROM t;\n");
}

#[test]
fn preserve_multi_word_keyword_order_by() {
    let cfg = FormatConfig {
        keyword_case: KeywordCase::Preserve,
        ..FormatConfig::default()
    };
    let mut fmt = Formatter::with_config(&cfg);
    let out = fmt
        .format("select id from foo OrDeR By id")
        .expect("formatting should succeed");
    assert_eq!(out, "select id from foo OrDeR By id;\n");
}

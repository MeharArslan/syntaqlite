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

/// Integration tests exercising the generated dispatch table + ctx
/// with the hand-written format_node and renderer.
use syntaqlite_runtime::fmt::{format_node, render, DocArena, FormatConfig, KeywordCase, LoadedFmt};

fn format_sql(sql: &str) -> String {
    format_sql_with(sql, &FormatConfig::default())
}

fn format_sql_with(sql: &str, config: &FormatConfig) -> String {
    let d = syntaqlite::dialect();
    let fmt = LoadedFmt::from_dialect(d).unwrap();
    let ni = syntaqlite_runtime::fmt::NodeInfo::from_dialect(d);
    let mut parser = syntaqlite::create_parser();
    let mut session = parser.parse(sql);
    let root = session.next_statement().unwrap().unwrap();
    let mut arena = DocArena::new();
    let doc = format_node(&fmt, &session, &ni, root, &mut arena);
    render(&arena, doc, config)
}

// -- Basic SELECT --

#[test]
fn select_literal() {
    assert_eq!(format_sql("SELECT 42"), "SELECT 42");
}

#[test]
fn select_column() {
    assert_eq!(format_sql("SELECT a FROM t"), "SELECT a FROM t");
}

#[test]
fn select_multiple_columns() {
    assert_eq!(format_sql("SELECT a, b, c FROM t"), "SELECT a, b, c FROM t");
}

#[test]
fn select_with_where() {
    assert_eq!(
        format_sql("SELECT a FROM t WHERE x = 1"),
        "SELECT a FROM t WHERE x = 1"
    );
}

#[test]
fn select_distinct() {
    assert_eq!(
        format_sql("SELECT DISTINCT a FROM t"),
        "SELECT DISTINCT a FROM t"
    );
}

#[test]
fn select_star() {
    assert_eq!(format_sql("SELECT * FROM t"), "SELECT * FROM t");
}

#[test]
fn select_column_alias() {
    assert_eq!(
        format_sql("SELECT a AS x FROM t"),
        "SELECT a AS x FROM t"
    );
}

// -- Line breaking --

#[test]
fn long_select_breaks() {
    let config = FormatConfig {
        line_width: 20,
        ..Default::default()
    };
    let result = format_sql_with("SELECT column_one, column_two FROM very_long_table", &config);
    assert_eq!(
        result,
        "SELECT\n  column_one,\n  column_two\nFROM very_long_table"
    );
}

// -- Expressions --

#[test]
fn binary_expr() {
    assert_eq!(format_sql("SELECT 1 + 2"), "SELECT 1 + 2");
}

#[test]
fn unary_expr() {
    assert_eq!(format_sql("SELECT -1"), "SELECT -1");
}

#[test]
fn binary_and_or() {
    assert_eq!(
        format_sql("SELECT a FROM t WHERE x = 1 AND y = 2"),
        "SELECT a FROM t WHERE x = 1 AND y = 2"
    );
}

// -- Table references --

#[test]
fn table_with_alias() {
    assert_eq!(
        format_sql("SELECT a FROM t AS u"),
        "SELECT a FROM t AS u"
    );
}

// -- Subqueries and complex clauses --

#[test]
fn select_with_group_by() {
    assert_eq!(
        format_sql("SELECT a, COUNT(*) FROM t GROUP BY a"),
        "SELECT a, COUNT(*) FROM t GROUP BY a"
    );
}

#[test]
fn select_with_order_by() {
    assert_eq!(
        format_sql("SELECT a FROM t ORDER BY a"),
        "SELECT a FROM t ORDER BY a"
    );
}

#[test]
fn select_with_limit() {
    assert_eq!(
        format_sql("SELECT a FROM t LIMIT 10"),
        "SELECT a FROM t LIMIT 10"
    );
}

// -- Other statement types --

#[test]
fn delete_stmt() {
    assert_eq!(
        format_sql("DELETE FROM t WHERE x = 1"),
        "DELETE FROM t WHERE x = 1"
    );
}

#[test]
fn update_stmt() {
    assert_eq!(
        format_sql("UPDATE t SET a = 1 WHERE x = 2"),
        "UPDATE t SET a = 1 WHERE x = 2"
    );
}

#[test]
fn insert_stmt() {
    assert_eq!(
        format_sql("INSERT INTO t(a, b) VALUES(1, 2)"),
        "INSERT INTO t(a, b) VALUES (1, 2)"
    );
}

#[test]
fn create_table() {
    assert_eq!(
        format_sql("CREATE TABLE t(a INTEGER, b TEXT)"),
        "CREATE TABLE t(a INTEGER, b TEXT)"
    );
}

#[test]
fn drop_table() {
    assert_eq!(format_sql("DROP TABLE t"), "DROP TABLE t");
}

#[test]
fn drop_table_if_exists() {
    assert_eq!(
        format_sql("DROP TABLE IF EXISTS t"),
        "DROP TABLE IF EXISTS t"
    );
}

// -- Line breaking for INSERT --

#[test]
fn insert_breaks_when_narrow() {
    let config = FormatConfig {
        line_width: 20,
        ..Default::default()
    };
    let result = format_sql_with("INSERT INTO t(a, b) VALUES(1, 2)", &config);
    assert_eq!(
        result,
        "INSERT INTO t(a, b)\nVALUES (1, 2)"
    );
}

// -- Large VALUES --

#[test]
fn insert_many_values_flat() {
    let config = FormatConfig {
        line_width: 40,
        ..Default::default()
    };
    let result = format_sql_with(
        "INSERT INTO t(a, b) VALUES(1, 2), (3, 4), (5, 6), (7, 8)",
        &config,
    );
    assert_eq!(
        result,
        "INSERT INTO t(a, b)\nVALUES (1, 2), (3, 4), (5, 6), (7, 8)"
    );
}

#[test]
fn insert_many_values_breaks() {
    let config = FormatConfig {
        line_width: 30,
        ..Default::default()
    };
    let result = format_sql_with(
        "INSERT INTO t(a, b) VALUES(1, 2), (3, 4), (5, 6), (7, 8)",
        &config,
    );
    assert_eq!(
        result,
        "INSERT INTO t(a, b)\nVALUES\n  (1, 2),\n  (3, 4),\n  (5, 6),\n  (7, 8)"
    );
}

// -- Keyword casing --

#[test]
fn keyword_case_lower() {
    let config = FormatConfig {
        keyword_case: KeywordCase::Lower,
        ..Default::default()
    };
    assert_eq!(
        format_sql_with("SELECT a FROM t", &config),
        "select a from t"
    );
}

#[test]
fn keyword_case_upper() {
    let config = FormatConfig {
        keyword_case: KeywordCase::Upper,
        ..Default::default()
    };
    assert_eq!(
        format_sql_with("select a from t", &config),
        "SELECT a FROM t"
    );
}

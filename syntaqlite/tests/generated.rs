/// Integration tests exercising the generated dispatch table + ctx
/// with the hand-written format_node and renderer.
use syntaqlite::sqlite::config::{FormatConfig, KeywordCase};

fn format_sql(sql: &str) -> String {
    format_sql_with(sql, FormatConfig::default())
}

fn format_sql_with(sql: &str, config: FormatConfig) -> String {
    let mut f = syntaqlite::fmt::Formatter::with_config(config).unwrap();
    let result = f.format(sql).unwrap();
    // Strip the trailing semicolon + newline that Formatter appends
    result
        .trim_end_matches('\n')
        .trim_end_matches(';')
        .to_string()
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
    assert_eq!(format_sql("SELECT a AS x FROM t"), "SELECT a AS x FROM t");
}

// -- Line breaking --

#[test]
fn long_select_breaks() {
    let config = FormatConfig {
        line_width: 20,
        ..Default::default()
    };
    let result = format_sql_with("SELECT column_one, column_two FROM very_long_table", config);
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
    assert_eq!(format_sql("SELECT a FROM t AS u"), "SELECT a FROM t AS u");
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
    let result = format_sql_with("INSERT INTO t(a, b) VALUES(1, 2)", config);
    assert_eq!(result, "INSERT INTO t(a, b)\nVALUES (1, 2)");
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
        config,
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
        config,
    );
    assert_eq!(
        result,
        "INSERT INTO t(a, b)\nVALUES\n  (1, 2),\n  (3, 4),\n  (5, 6),\n  (7, 8)"
    );
}

// -- Comments --

// Bug 2: Leading line comment concatenates with next token (missing newline after comment)
#[test]
fn comment_trailing_on_select() {
    // Trailing comment on same line as SELECT — should stay on that line
    assert_eq!(
        format_sql("SELECT -- pick cols\na FROM t"),
        "SELECT -- pick cols\n  a\nFROM t"
    );
}

#[test]
fn comment_leading_before_column() {
    // Comment on its own line before a column — should not merge with the column
    let config = FormatConfig {
        line_width: 20,
        ..Default::default()
    };
    assert_eq!(
        format_sql_with("SELECT\n  -- comment\n  a\nFROM t", config),
        "SELECT\n  -- comment\n  a\nFROM t"
    );
}

#[test]
fn comment_between_columns() {
    // Comment between two columns in a broken select list
    let config = FormatConfig {
        line_width: 20,
        ..Default::default()
    };
    assert_eq!(
        format_sql_with("SELECT\n  a,\n  -- about b\n  b\nFROM t", config),
        "SELECT\n  a,\n  -- about b\n  b\nFROM t"
    );
}

#[test]
fn comment_before_join_does_not_move() {
    // A comment between child(left) and JOIN should stay before JOIN,
    // not get pulled to after JOIN by child(right)'s drain.
    assert_eq!(
        format_sql("SELECT a FROM slice\n-- before join\nJOIN track"),
        "SELECT a\nFROM slice\n-- before join\nJOIN track"
    );
}

#[test]
fn comment_after_star_column() {
    // SELECT * produces a ResultColumn with no Span fields (just a keyword).
    // Comments after * should not be orphaned.
    assert_eq!(
        format_sql("SELECT *\n-- about from\nFROM t"),
        "SELECT *\n-- about from\nFROM t"
    );
}

#[test]
fn comment_trailing_not_dropped_when_followed_by_line_comment() {
    // A trailing comment (-- x) after a keyword's last token should not be
    // dropped when there is another line comment (-- z) between it and the
    // next keyword.  The gap check must skip over comment regions.
    assert_eq!(
        format_sql("select a, b\n-- y\nfrom t -- x\n-- z\nwhere c = 1"),
        "SELECT a, b\n-- y\nFROM t -- x\n-- z\nWHERE\n  c = 1"
    );
}

// -- Multi-statement comments --

#[test]
fn multi_stmt_basic() {
    assert_eq!(format_sql("SELECT 1;\nSELECT 2"), "SELECT 1;\n\nSELECT 2");
}

#[test]
fn multi_stmt_comment_between() {
    // A line comment between two statements should be preserved.
    assert_eq!(
        format_sql("SELECT 1;\n-- between\nSELECT 2"),
        "SELECT 1;\n\n-- between\nSELECT 2"
    );
}

#[test]
fn multi_stmt_trailing_comment_after_first() {
    // Trailing comment on the same line as the semicolon after stmt 1.
    assert_eq!(
        format_sql("SELECT 1; -- after first\nSELECT 2"),
        "SELECT 1; -- after first\n\nSELECT 2"
    );
}

#[test]
fn comment_before_first_stmt() {
    // A leading comment before the very first statement.
    assert_eq!(format_sql("-- header\nSELECT 1"), "-- header\nSELECT 1");
}

// -- Keyword casing --

#[test]
fn keyword_case_lower() {
    let config = FormatConfig {
        keyword_case: KeywordCase::Lower,
        ..Default::default()
    };
    assert_eq!(
        format_sql_with("SELECT a FROM t", config),
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
        format_sql_with("select a from t", config),
        "SELECT a FROM t"
    );
}

/// Integration tests exercising the generated dispatch table + ctx
/// with the hand-written format_node and renderer.
use syntaqlite::fmt::{FormatConfig, KeywordCase};

fn format_sql(sql: &str) -> String {
    format_sql_with(sql, FormatConfig::default())
}

fn format_sql_with(sql: &str, config: FormatConfig) -> String {
    let dialect = syntaqlite::dialect::sqlite();
    let mut f = syntaqlite::Formatter::builder(dialect)
        .format_config(config)
        .build();
    let result = f.format(sql).unwrap();
    // Strip the trailing semicolon + newline that Formatter appends
    result
        .trim_end_matches('\n')
        .trim_end_matches(';')
        .to_string()
}

// -- Idempotent round-trip: well-formatted SQL survives formatting unchanged --

#[test]
fn format_idempotent() {
    let cases = [
        // Basic SELECT variations
        "SELECT 42",
        "SELECT a FROM t",
        "SELECT a, b, c FROM t",
        "SELECT a FROM t WHERE x = 1",
        "SELECT DISTINCT a FROM t",
        "SELECT * FROM t",
        "SELECT a AS x FROM t",
        // Expressions
        "SELECT 1 + 2",
        "SELECT -1",
        "SELECT a FROM t WHERE x = 1 AND y = 2",
        // Table references
        "SELECT a FROM t AS u",
        // Clauses
        "SELECT a, COUNT(*) FROM t GROUP BY a",
        "SELECT a FROM t ORDER BY a",
        "SELECT a FROM t LIMIT 10",
        // Other statement types
        "DELETE FROM t WHERE x = 1",
        "UPDATE t SET a = 1 WHERE x = 2",
        "CREATE TABLE t(a INTEGER, b TEXT)",
        "DROP TABLE t",
        "DROP TABLE IF EXISTS t",
    ];
    for sql in &cases {
        assert_eq!(format_sql(sql), *sql, "round-trip failed for: {sql}");
    }
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

// -- Formatting transformations (input != output) --

fn format_sql_with_cflags(sql: &str, config: FormatConfig, cflag_indices: &[u32]) -> String {
    use syntaqlite_parser::DialectConfig;
    let mut dc = DialectConfig::default();
    for &idx in cflag_indices {
        dc.cflags.set(idx);
    }
    let dialect = syntaqlite::dialect::sqlite();
    let mut f = syntaqlite::Formatter::builder(dialect)
        .format_config(config)
        .dialect_config(dc)
        .build();
    let result = f.format(sql).unwrap();
    result
        .trim_end_matches('\n')
        .trim_end_matches(';')
        .to_string()
}

#[test]
fn delete_with_order_by_limit() {
    assert_eq!(
        format_sql_with_cflags(
            "delete from t where x > 0 order by id limit 10 offset 5",
            FormatConfig::default(),
            &[40], // SQLITE_ENABLE_UPDATE_DELETE_LIMIT
        ),
        "DELETE FROM t WHERE x > 0 ORDER BY id LIMIT 10 OFFSET 5"
    );
}

#[test]
fn update_with_order_by_limit() {
    assert_eq!(
        format_sql_with_cflags(
            "update t set a = 1 where x > 0 order by id limit 5",
            FormatConfig::default(),
            &[40], // SQLITE_ENABLE_UPDATE_DELETE_LIMIT
        ),
        "UPDATE t SET a = 1 WHERE x > 0 ORDER BY id LIMIT 5"
    );
}

#[test]
fn insert_stmt() {
    assert_eq!(
        format_sql("INSERT INTO t(a, b) VALUES(1, 2)"),
        "INSERT INTO t(a, b) VALUES (1, 2)"
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

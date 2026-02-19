use syntaqlite_runtime::fmt::{render, DocArena, FormatConfig, KeywordCase, NIL_DOC};

fn default_config() -> FormatConfig {
    FormatConfig::default()
}

fn narrow_config(width: usize) -> FormatConfig {
    FormatConfig {
        line_width: width,
        ..Default::default()
    }
}

/// Helper: build a comma-separated list with Line breaks between items.
/// Returns a group containing: item1 "," line item2 "," line item3 ...
fn comma_list<'a>(arena: &mut DocArena<'a>, items: &[&'a str]) -> u32 {
    let mut parts = NIL_DOC;
    for (i, item) in items.iter().enumerate() {
        let t = arena.text(item);
        if i > 0 {
            let comma = arena.text(",");
            let sp = arena.line();
            parts = arena.cats(&[parts, comma, sp, t]);
        } else {
            parts = t;
        }
    }
    parts
}

#[test]
fn short_select_fits_one_line() {
    let mut arena = DocArena::new();
    let select = arena.keyword("SELECT");
    let sp1 = arena.text(" ");
    let cols = comma_list(&mut arena, &["a", "b"]);
    let sp2 = arena.line();
    let from = arena.keyword("FROM");
    let sp3 = arena.text(" ");
    let table = arena.text("t");

    let inner = arena.cats(&[select, sp1, cols, sp2, from, sp3, table]);
    let doc = arena.group(inner);

    assert_eq!(render(&arena, doc, &default_config()), "SELECT a, b FROM t");
}

#[test]
fn long_select_breaks_with_indentation() {
    let mut arena = DocArena::new();
    let select = arena.keyword("SELECT");
    let sp1 = arena.line();
    let cols = comma_list(
        &mut arena,
        &["column_one", "column_two", "column_three", "column_four"],
    );
    let cols_body = arena.cat(sp1, cols);
    let cols_nested = arena.nest(4, cols_body);
    let sp2 = arena.line();
    let from = arena.keyword("FROM");
    let sp3 = arena.text(" ");
    let table = arena.text("very_long_table_name");

    let inner = arena.cats(&[select, cols_nested, sp2, from, sp3, table]);
    let doc = arena.group(inner);

    let result = render(&arena, doc, &narrow_config(40));
    assert_eq!(
        result,
        "SELECT\n    column_one,\n    column_two,\n    column_three,\n    column_four\nFROM very_long_table_name"
    );
}

#[test]
fn multi_statement_with_hardline() {
    let mut arena = DocArena::new();

    let s1_select = arena.keyword("SELECT");
    let s1_sp = arena.text(" ");
    let s1_val = arena.text("1");
    let s1_semi = arena.text(";");
    let stmt1 = arena.cats(&[s1_select, s1_sp, s1_val, s1_semi]);

    let hl = arena.hardline();

    let s2_select = arena.keyword("SELECT");
    let s2_sp = arena.text(" ");
    let s2_val = arena.text("2");
    let s2_semi = arena.text(";");
    let stmt2 = arena.cats(&[s2_select, s2_sp, s2_val, s2_semi]);

    let doc = arena.cats(&[stmt1, hl, stmt2]);
    assert_eq!(
        render(&arena, doc, &default_config()),
        "SELECT 1;\nSELECT 2;"
    );
}

#[test]
fn nested_groups_subquery_in_parens() {
    let mut arena = DocArena::new();

    let select = arena.keyword("SELECT");
    let sp = arena.text(" ");
    let star = arena.text("*");
    let line1 = arena.line();
    let from = arena.keyword("FROM");
    let sp2 = arena.text(" ");

    let lp = arena.text("(");
    let sub_select = arena.keyword("SELECT");
    let sub_sp = arena.text(" ");
    let sub_cols = comma_list(&mut arena, &["a", "b"]);
    let sub_line = arena.line();
    let sub_from = arena.keyword("FROM");
    let sub_sp2 = arena.text(" ");
    let sub_table = arena.text("t");
    let sub_inner = arena.cats(&[
        sub_select, sub_sp, sub_cols, sub_line, sub_from, sub_sp2, sub_table,
    ]);
    let sub_group = arena.group(sub_inner);

    let sl = arena.softline();
    let nested_body = arena.cats(&[lp, sl, sub_group]);
    let paren_nested = arena.nest(4, nested_body);
    let sl2 = arena.softline();
    let rp = arena.text(")");
    let paren_doc = arena.cats(&[paren_nested, sl2, rp]);
    let paren_group = arena.group(paren_doc);

    let as_kw = arena.text(" ");
    let as_text = arena.keyword("AS");
    let as_sp = arena.text(" ");
    let alias = arena.text("sub");

    let outer_inner = arena.cats(&[
        select, sp, star, line1, from, sp2, paren_group, as_kw, as_text, as_sp, alias,
    ]);
    let doc = arena.group(outer_inner);

    assert_eq!(
        render(&arena, doc, &default_config()),
        "SELECT * FROM (SELECT a, b FROM t) AS sub"
    );

    let result = render(&arena, doc, &narrow_config(24));
    assert_eq!(
        result,
        "SELECT *\nFROM (\n    SELECT a, b FROM t\n) AS sub"
    );
}

#[test]
fn keyword_casing_integration() {
    let mut arena = DocArena::new();
    let select = arena.keyword("select");
    let sp = arena.text(" ");
    let col = arena.text("name");
    let line = arena.line();
    let from = arena.keyword("from");
    let sp2 = arena.text(" ");
    let table = arena.text("users");

    let inner = arena.cats(&[select, sp, col, line, from, sp2, table]);
    let doc = arena.group(inner);

    let upper_config = FormatConfig {
        keyword_case: KeywordCase::Upper,
        ..Default::default()
    };
    assert_eq!(
        render(&arena, doc, &upper_config),
        "SELECT name FROM users"
    );
}

#[test]
fn line_suffix_trailing_comment() {
    let mut arena = DocArena::new();

    let select = arena.keyword("SELECT");
    let sp = arena.text(" ");
    let val = arena.text("1");

    let comment_space = arena.text(" ");
    let comment = arena.text("-- pick one");
    let comment_doc = arena.cat(comment_space, comment);
    let suffix = arena.line_suffix(comment_doc);

    let line = arena.hardline();
    let from = arena.keyword("FROM");
    let sp2 = arena.text(" ");
    let table = arena.text("t");

    let doc = arena.cats(&[select, sp, val, suffix, line, from, sp2, table]);
    assert_eq!(
        render(&arena, doc, &default_config()),
        "SELECT 1 -- pick one\nFROM t"
    );
}

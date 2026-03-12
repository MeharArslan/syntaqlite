// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Integration tests: macro regions are emitted verbatim by the formatter.

fn formatter() -> syntaqlite::Formatter {
    syntaqlite::Formatter::new()
}

#[test]
fn macro_call_emitted_verbatim() {
    let mut fmt = formatter();
    let out = fmt.format("SELECT foo!(1 + 2), 3").expect("format failed");
    assert_eq!(out, "SELECT foo!(1 + 2), 3;\n");
}

#[test]
fn macro_multi_node_emitted_once() {
    let mut fmt = formatter();
    let out = fmt.format("SELECT macro!(a, b)").expect("format failed");
    assert_eq!(out, "SELECT macro!(a, b);\n");
}

#[test]
fn macro_multi_node_no_extra_separator() {
    let mut fmt = formatter();
    let out = fmt.format("SELECT foo!(a, b), c").expect("format failed");
    assert_eq!(out, "SELECT foo!(a, b), c;\n");
}

#[test]
fn macro_multiline_reindented() {
    let input = concat!(
        "SELECT *\n",
        "FROM graph_next_sibling!(\n",
        "        (\n",
        "          SELECT id, parent_id, ts\n",
        "          FROM slice\n",
        "          WHERE dur = 0\n",
        "        )\n",
        "    )\n",
    );
    let mut fmt = syntaqlite::Formatter::new();
    let out = fmt.format(input).expect("format failed");
    eprintln!("=== actual ===\n{out}=== end ===");
    assert_eq!(
        out,
        concat!(
            "SELECT *\n",
            "FROM graph_next_sibling!(\n",
            "  (\n",
            "    SELECT id, parent_id, ts\n",
            "    FROM slice\n",
            "    WHERE dur = 0\n",
            "  )\n",
            ");\n",
        )
    );
}

#[test]
fn macro_parens_in_strings_ignored() {
    // Parens inside string literals must not affect indentation depth.
    let input = concat!(
        "SELECT *\n",
        "FROM my_macro!(\n",
        "  (\n",
        "    SELECT '(((' AS x\n",
        "    FROM t\n",
        "  )\n",
        ")\n",
    );
    let mut fmt = syntaqlite::Formatter::new();
    let out = fmt.format(input).expect("format failed");
    eprintln!("=== actual ===\n{out}=== end ===");
    assert_eq!(
        out,
        concat!(
            "SELECT *\n",
            "FROM my_macro!(\n",
            "  (\n",
            "    SELECT '(((' AS x\n",
            "    FROM t\n",
            "  )\n",
            ");\n",
        )
    );
}

#[test]
fn macro_with_function_calls() {
    // IIF() and other function calls with parens must be tracked correctly.
    let input = concat!(
        "SELECT *\n",
        "FROM scan!(\n",
        "  (\n",
        "    SELECT\n",
        "      IIF(\n",
        "        x > 0,\n",
        "        1,\n",
        "        0\n",
        "      ) AS flag\n",
        "    FROM t\n",
        "  )\n",
        ")\n",
    );
    let mut fmt = syntaqlite::Formatter::new();
    let out = fmt.format(input).expect("format failed");
    eprintln!("=== actual ===\n{out}=== end ===");
    assert_eq!(
        out,
        concat!(
            "SELECT *\n",
            "FROM scan!(\n",
            "  (\n",
            "    SELECT\n",
            "    IIF(\n",
            "      x > 0,\n",
            "      1,\n",
            "      0\n",
            "    ) AS flag\n",
            "    FROM t\n",
            "  )\n",
            ");\n",
        )
    );
}

#[test]
fn macro_comma_separated_args() {
    // Multiple macro arguments at different paren depths.
    let input = concat!(
        "SELECT *\n",
        "FROM scan!(\n",
        "    edges,\n",
        "    inits,\n",
        "    (a, b, c),\n",
        "    (\n",
        "      SELECT id\n",
        "      FROM t\n",
        "    )\n",
        "  )\n",
    );
    let mut fmt = syntaqlite::Formatter::new();
    let out = fmt.format(input).expect("format failed");
    eprintln!("=== actual ===\n{out}=== end ===");
    assert_eq!(
        out,
        concat!(
            "SELECT *\n",
            "FROM scan!(\n",
            "  edges,\n",
            "  inits,\n",
            "  (a, b, c),\n",
            "  (\n",
            "    SELECT id\n",
            "    FROM t\n",
            "  )\n",
            ");\n",
        )
    );
}

#[test]
fn macro_in_frame_bound_preserves_following() {
    let input = "SELECT count() OVER (ORDER BY ts RANGE BETWEEN CURRENT ROW AND my_macro!(x) FOLLOWING) FROM t;\n";
    let mut fmt = syntaqlite::Formatter::new();
    let out = fmt.format(input).expect("format failed");
    eprintln!("=== actual ===\n{out}=== end ===");
    assert!(
        out.contains("FOLLOWING"),
        "FOLLOWING keyword was dropped: {out}"
    );
}

#[test]
fn macro_single_line_preserved() {
    let input = "SELECT foo!(1 + 2), 3\n";
    let mut fmt = syntaqlite::Formatter::new();
    let out = fmt.format(input).expect("format failed");
    eprintln!("=== actual ===\n{out}=== end ===");
    assert_eq!(out, "SELECT foo!(1 + 2), 3;\n");
}

#[test]
fn no_macro_regions_formats_normally() {
    let mut fmt = formatter();
    let out = fmt.format("SELECT  1+2,  3").expect("format failed");
    assert_eq!(out, "SELECT 1 + 2, 3;\n");
}

# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

from python.dev.diff_tests.testing import DiffTestBlueprint, TestSuite


class PerfettoIndexFormat(TestSuite):
    def test_create_perfetto_index_multiline_columns(self):
        return DiffTestBlueprint(
            sql="""\
                CREATE PERFETTO INDEX idx_hot_slices ON hot_slices(ts,
                dur,
                track_id)
            """,
            out="CREATE PERFETTO INDEX idx_hot_slices ON hot_slices(ts, dur, track_id)",
        )


class PerfettoMacroFormat(TestSuite):
    def test_create_perfetto_macro_body_preserved(self):
        return DiffTestBlueprint(
            sql="""\
                CREATE PERFETTO MACRO m(x TableOrSubquery) RETURNS TableOrSubquery AS x
            """,
            out="""\
                CREATE PERFETTO MACRO m(x TableOrSubquery)
                RETURNS TableOrSubquery
                AS x
            """,
        )

    def test_create_perfetto_macro_body_select(self):
        return DiffTestBlueprint(
            sql="""\
                CREATE PERFETTO MACRO my_macro(t TableOrSubquery) RETURNS TableOrSubquery AS (SELECT * FROM $t)
            """,
            out="""\
                CREATE PERFETTO MACRO my_macro(t TableOrSubquery)
                RETURNS TableOrSubquery
                AS (SELECT * FROM $t)
            """,
        )

    def test_create_perfetto_macro_long_args_indented(self):
        return DiffTestBlueprint(
            sql="""\
                CREATE PERFETTO MACRO _viz_flamegraph_filter_frames(source TableOrSubquery, show_from_frame_bits Expr) RETURNS TableOrSubquery AS $source
            """,
            out="""\
                CREATE PERFETTO MACRO _viz_flamegraph_filter_frames(
                  source TableOrSubquery,
                  show_from_frame_bits Expr
                )
                RETURNS TableOrSubquery
                AS $source
            """,
        )

    def test_create_or_replace_perfetto_macro_body(self):
        return DiffTestBlueprint(
            sql="""\
                CREATE OR REPLACE PERFETTO MACRO m(x Expr) RETURNS Expr AS $x
            """,
            out="""\
                CREATE OR REPLACE PERFETTO MACRO m(x Expr)
                RETURNS Expr
                AS $x
            """,
        )


class PerfettoMacroCallFormat(TestSuite):
    def test_macro_call_in_select(self):
        return DiffTestBlueprint(
            sql="SELECT foo!(1 + 2), 3",
            out="SELECT foo!(1 + 2), 3",
        )

    def test_macro_call_in_from(self):
        return DiffTestBlueprint(
            sql="SELECT * FROM my_macro!(t1)",
            out="SELECT * FROM my_macro!(t1)",
        )

    def test_macro_call_nested_parens(self):
        return DiffTestBlueprint(
            sql="SELECT * FROM graph_reachable_dfs!((SELECT id FROM t), (SELECT id FROM s))",
            out="SELECT * FROM graph_reachable_dfs!((SELECT id FROM t), (SELECT id FROM s))",
        )

    def test_macro_call_no_args(self):
        return DiffTestBlueprint(
            sql="SELECT my_macro!()",
            out="SELECT my_macro!()",
        )

    def test_macro_call_multi_node(self):
        return DiffTestBlueprint(
            sql="SELECT my_fn!(a, b)",
            out="SELECT my_fn!(a, b)",
        )

    def test_macro_call_multi_node_no_extra_separator(self):
        return DiffTestBlueprint(
            sql="SELECT foo!(a, b), c",
            out="SELECT foo!(a, b), c",
        )

    def test_macro_multiline_reindented(self):
        return DiffTestBlueprint(
            sql="""\
                SELECT *
                FROM graph_next_sibling!(
                        (
                          SELECT id, parent_id, ts
                          FROM slice
                          WHERE dur = 0
                        )
                    )
            """,
            out="""\
                SELECT *
                FROM graph_next_sibling!(
                  (
                    SELECT id, parent_id, ts
                    FROM slice
                    WHERE dur = 0
                  )
                )
            """,
        )

    def test_macro_parens_in_strings_ignored(self):
        return DiffTestBlueprint(
            sql="""\
                SELECT *
                FROM my_macro!(
                  (
                    SELECT '(((' AS x
                    FROM t
                  )
                )
            """,
            out="""\
                SELECT *
                FROM my_macro!(
                  (
                    SELECT '(((' AS x
                    FROM t
                  )
                )
            """,
        )

    def test_macro_with_function_calls(self):
        return DiffTestBlueprint(
            sql="""\
                SELECT *
                FROM scan!(
                  (
                    SELECT
                      IIF(
                        x > 0,
                        1,
                        0
                      ) AS flag
                    FROM t
                  )
                )
            """,
            out="""\
                SELECT *
                FROM scan!(
                  (
                    SELECT
                    IIF(
                      x > 0,
                      1,
                      0
                    ) AS flag
                    FROM t
                  )
                )
            """,
        )

    def test_macro_comma_separated_args(self):
        return DiffTestBlueprint(
            sql="""\
                SELECT *
                FROM scan!(
                    edges,
                    inits,
                    (a, b, c),
                    (
                      SELECT id
                      FROM t
                    )
                  )
            """,
            out="""\
                SELECT *
                FROM scan!(
                  edges,
                  inits,
                  (a, b, c),
                  (
                    SELECT id
                    FROM t
                  )
                )
            """,
        )

    def test_macro_in_frame_bound_preserves_following(self):
        return DiffTestBlueprint(
            sql="SELECT count() OVER (ORDER BY ts RANGE BETWEEN CURRENT ROW AND my_macro!(x) FOLLOWING) FROM t",
            out="""\
                SELECT
                  count() OVER (
                    ORDER BY ts
                    RANGE BETWEEN CURRENT ROW AND my_macro!(x) FOLLOWING
                  )
                FROM t
            """,
        )


    def test_macro_partition_by_multi_arg_nests(self):
        return DiffTestBlueprint(
            sql="""\
                SELECT last_value(thread.start_ts) OVER (
                  PARTITION BY upid, android_standardize_thread_name(thread.name)
                  ORDER BY thread.start_ts
                  RANGE BETWEEN CURRENT ROW AND cast_int!($sliding_window_dur) FOLLOWING
                ) FROM thread
            """,
            out="""\
                SELECT
                  last_value(thread.start_ts) OVER (
                    PARTITION BY
                      upid,
                      android_standardize_thread_name(thread.name)
                    ORDER BY thread.start_ts
                    RANGE BETWEEN CURRENT ROW AND cast_int!($sliding_window_dur) FOLLOWING
                  )
                FROM thread
            """,
        )


class PerfettoFunctionFormat(TestSuite):
    def test_create_perfetto_function_returns_on_newline(self):
        return DiffTestBlueprint(
            sql="""\
                CREATE PERFETTO FUNCTION top_slice_count(cpu INT, min_dur INT) RETURNS INT AS
                SELECT count(*) FROM slice WHERE cpu = $cpu AND dur >= $min_dur;
            """,
            out="""\
                CREATE PERFETTO FUNCTION top_slice_count(cpu INT, min_dur INT)
                RETURNS INT
                AS
                SELECT count(*) FROM slice WHERE cpu = $cpu AND dur >= $min_dur
            """,
        )

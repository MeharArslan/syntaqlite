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

# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

from python.syntaqlite.diff_tests.testing import DiffTestBlueprint, TestSuite


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

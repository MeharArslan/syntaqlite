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

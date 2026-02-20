# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Amalgamation integration tests for perfetto dialect extension.

These tests verify that dialect extensions (additional .y and .synq files)
are correctly merged with the base SQLite grammar and produce a working
amalgamated parser.
"""

from python.syntaqlite.diff_tests.testing import AstTestBlueprint, TestSuite


class PerfettoExtension(TestSuite):
    """Tests for perfetto dialect extension syntax."""

    def test_create_perfetto_table(self):
        """Extension keyword PERFETTO is recognized by the tokenizer."""
        return AstTestBlueprint(
            sql="CREATE PERFETTO TABLE foo",
            out="""\
            CreatePerfettoTableStmt
              table_name: "foo"
""",
        )

    def test_base_select_still_works(self):
        """Base SQLite syntax must still work in an extended dialect."""
        return AstTestBlueprint(
            sql="SELECT 1",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      Literal
                        literal_type: INTEGER
                        source: "1"
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_base_create_table_still_works(self):
        """Regular CREATE TABLE must coexist with CREATE PERFETTO TABLE."""
        return AstTestBlueprint(
            sql="CREATE TABLE t (id INTEGER)",
            out="""\
            CreateTableStmt
              table_name: "t"
              schema: null
              is_temp: FALSE
              if_not_exists: FALSE
              flags: (none)
              columns:
                ColumnDefList [1 items]
                  ColumnDef
                    column_name: "id"
                    type_name: "INTEGER"
                    constraints: (none)
              table_constraints: (none)
              as_select: (none)
""",
        )

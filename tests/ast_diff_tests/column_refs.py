# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Column reference expression AST tests."""

from python.syntaqlite.diff_tests.testing import DiffTestBlueprint, TestSuite


class ColumnRefBasic(TestSuite):
    """Column reference tests."""

    def test_simple_column(self):
        return DiffTestBlueprint(
            sql="SELECT x",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: (none)
                    expr:
                      ColumnRef
                        column: "x"
                        table: (none)
                        schema: (none)
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_qualified_column(self):
        return DiffTestBlueprint(
            sql="SELECT t.x",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: (none)
                    expr:
                      ColumnRef
                        column: "x"
                        table: "t"
                        schema: (none)
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_fully_qualified_column(self):
        return DiffTestBlueprint(
            sql="SELECT s.t.x",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: (none)
                    expr:
                      ColumnRef
                        column: "x"
                        table: "t"
                        schema: "s"
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_multiple_columns(self):
        return DiffTestBlueprint(
            sql="SELECT a, b, c",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [3 items]
                  ResultColumn
                    flags: (none)
                    alias: (none)
                    expr:
                      ColumnRef
                        column: "a"
                        table: (none)
                        schema: (none)
                  ResultColumn
                    flags: (none)
                    alias: (none)
                    expr:
                      ColumnRef
                        column: "b"
                        table: (none)
                        schema: (none)
                  ResultColumn
                    flags: (none)
                    alias: (none)
                    expr:
                      ColumnRef
                        column: "c"
                        table: (none)
                        schema: (none)
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_table_star(self):
        return DiffTestBlueprint(
            sql="SELECT t.*",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: STAR
                    alias:
                      IdentName
                        source: "t"
                    expr: (none)
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_mixed_expressions(self):
        return DiffTestBlueprint(
            sql="SELECT a, t.b, 1 + x",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [3 items]
                  ResultColumn
                    flags: (none)
                    alias: (none)
                    expr:
                      ColumnRef
                        column: "a"
                        table: (none)
                        schema: (none)
                  ResultColumn
                    flags: (none)
                    alias: (none)
                    expr:
                      ColumnRef
                        column: "b"
                        table: "t"
                        schema: (none)
                  ResultColumn
                    flags: (none)
                    alias: (none)
                    expr:
                      BinaryExpr
                        op: PLUS
                        left:
                          Literal
                            literal_type: INTEGER
                            source: "1"
                        right:
                          ColumnRef
                            column: "x"
                            table: (none)
                            schema: (none)
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

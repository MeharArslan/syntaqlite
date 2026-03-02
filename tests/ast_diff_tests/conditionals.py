# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Conditional expression AST tests: IS, BETWEEN, LIKE, CASE."""

from python.syntaqlite.diff_tests.testing import DiffTestBlueprint, TestSuite


class IsExprBasic(TestSuite):
    """IS/ISNULL/NOTNULL expression tests."""

    def test_isnull(self):
        return DiffTestBlueprint(
            sql="SELECT 1 ISNULL",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      IsExpr
                        op: ISNULL
                        left:
                          Literal
                            literal_type: INTEGER
                            source: "1"
                        right: (none)
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_notnull(self):
        return DiffTestBlueprint(
            sql="SELECT 1 NOTNULL",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      IsExpr
                        op: NOTNULL
                        left:
                          Literal
                            literal_type: INTEGER
                            source: "1"
                        right: (none)
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_not_null(self):
        return DiffTestBlueprint(
            sql="SELECT 1 NOT NULL",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      IsExpr
                        op: NOTNULL
                        left:
                          Literal
                            literal_type: INTEGER
                            source: "1"
                        right: (none)
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_is_null(self):
        return DiffTestBlueprint(
            sql="SELECT 1 IS NULL",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      IsExpr
                        op: IS
                        left:
                          Literal
                            literal_type: INTEGER
                            source: "1"
                        right:
                          Literal
                            literal_type: NULL
                            source: "NULL"
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_is_not_null(self):
        return DiffTestBlueprint(
            sql="SELECT 1 IS NOT NULL",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      IsExpr
                        op: IS_NOT
                        left:
                          Literal
                            literal_type: INTEGER
                            source: "1"
                        right:
                          Literal
                            literal_type: NULL
                            source: "NULL"
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_is_expr(self):
        return DiffTestBlueprint(
            sql="SELECT 1 IS 2",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      IsExpr
                        op: IS
                        left:
                          Literal
                            literal_type: INTEGER
                            source: "1"
                        right:
                          Literal
                            literal_type: INTEGER
                            source: "2"
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_is_not_expr(self):
        return DiffTestBlueprint(
            sql="SELECT 1 IS NOT 2",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      IsExpr
                        op: IS_NOT
                        left:
                          Literal
                            literal_type: INTEGER
                            source: "1"
                        right:
                          Literal
                            literal_type: INTEGER
                            source: "2"
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_is_not_distinct_from(self):
        return DiffTestBlueprint(
            sql="SELECT 1 IS NOT DISTINCT FROM 2",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      IsExpr
                        op: IS_NOT_DISTINCT
                        left:
                          Literal
                            literal_type: INTEGER
                            source: "1"
                        right:
                          Literal
                            literal_type: INTEGER
                            source: "2"
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_is_distinct_from(self):
        return DiffTestBlueprint(
            sql="SELECT 1 IS DISTINCT FROM 2",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      IsExpr
                        op: IS_DISTINCT
                        left:
                          Literal
                            literal_type: INTEGER
                            source: "1"
                        right:
                          Literal
                            literal_type: INTEGER
                            source: "2"
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )


class BetweenExprBasic(TestSuite):
    """BETWEEN expression tests."""

    def test_between(self):
        return DiffTestBlueprint(
            sql="SELECT 1 BETWEEN 0 AND 10",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      BetweenExpr
                        negated: FALSE
                        operand:
                          Literal
                            literal_type: INTEGER
                            source: "1"
                        low:
                          Literal
                            literal_type: INTEGER
                            source: "0"
                        high:
                          Literal
                            literal_type: INTEGER
                            source: "10"
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_not_between(self):
        return DiffTestBlueprint(
            sql="SELECT 1 NOT BETWEEN 0 AND 10",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      BetweenExpr
                        negated: TRUE
                        operand:
                          Literal
                            literal_type: INTEGER
                            source: "1"
                        low:
                          Literal
                            literal_type: INTEGER
                            source: "0"
                        high:
                          Literal
                            literal_type: INTEGER
                            source: "10"
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )


class LikeExprBasic(TestSuite):
    """LIKE expression tests."""

    def test_like(self):
        return DiffTestBlueprint(
            sql="SELECT 'abc' LIKE 'a%'",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      LikeExpr
                        negated: FALSE
                        operand:
                          Literal
                            literal_type: STRING
                            source: "'abc'"
                        pattern:
                          Literal
                            literal_type: STRING
                            source: "'a%'"
                        escape: (none)
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_not_like(self):
        return DiffTestBlueprint(
            sql="SELECT 'abc' NOT LIKE 'a%'",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      LikeExpr
                        negated: TRUE
                        operand:
                          Literal
                            literal_type: STRING
                            source: "'abc'"
                        pattern:
                          Literal
                            literal_type: STRING
                            source: "'a%'"
                        escape: (none)
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_like_escape(self):
        return DiffTestBlueprint(
            sql="SELECT 'abc' LIKE 'a%' ESCAPE '\\'",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      LikeExpr
                        negated: FALSE
                        operand:
                          Literal
                            literal_type: STRING
                            source: "'abc'"
                        pattern:
                          Literal
                            literal_type: STRING
                            source: "'a%'"
                        escape:
                          Literal
                            literal_type: STRING
                            source: "'\\'"
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )


class CaseExprBasic(TestSuite):
    """CASE expression tests."""

    def test_simple_case(self):
        return DiffTestBlueprint(
            sql="SELECT CASE WHEN 1 THEN 'a' ELSE 'b' END",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      CaseExpr
                        operand: (none)
                        else_expr:
                          Literal
                            literal_type: STRING
                            source: "'b'"
                        whens:
                          CaseWhenList [1 items]
                            CaseWhen
                              when_expr:
                                Literal
                                  literal_type: INTEGER
                                  source: "1"
                              then_expr:
                                Literal
                                  literal_type: STRING
                                  source: "'a'"
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_case_with_operand(self):
        return DiffTestBlueprint(
            sql="SELECT CASE 1 WHEN 1 THEN 'yes' WHEN 2 THEN 'no' END",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      CaseExpr
                        operand:
                          Literal
                            literal_type: INTEGER
                            source: "1"
                        else_expr: (none)
                        whens:
                          CaseWhenList [2 items]
                            CaseWhen
                              when_expr:
                                Literal
                                  literal_type: INTEGER
                                  source: "1"
                              then_expr:
                                Literal
                                  literal_type: STRING
                                  source: "'yes'"
                            CaseWhen
                              when_expr:
                                Literal
                                  literal_type: INTEGER
                                  source: "2"
                              then_expr:
                                Literal
                                  literal_type: STRING
                                  source: "'no'"
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_case_no_else(self):
        return DiffTestBlueprint(
            sql="SELECT CASE WHEN 1 THEN 'a' END",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: null
                    expr:
                      CaseExpr
                        operand: (none)
                        else_expr: (none)
                        whens:
                          CaseWhenList [1 items]
                            CaseWhen
                              when_expr:
                                Literal
                                  literal_type: INTEGER
                                  source: "1"
                              then_expr:
                                Literal
                                  literal_type: STRING
                                  source: "'a'"
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

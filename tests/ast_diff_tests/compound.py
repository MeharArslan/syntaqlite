# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Compound SELECT, subquery, and IN expression AST tests."""

from python.syntaqlite.diff_tests.testing import DiffTestBlueprint, TestSuite


class CompoundSelect(TestSuite):
    """Compound SELECT statement tests."""

    def test_union(self):
        return DiffTestBlueprint(
            sql="SELECT 1 UNION SELECT 2",
            out="""\
            CompoundSelect
              op: UNION
              left:
                SelectStmt
                  flags: (none)
                  columns:
                    ResultColumnList [1 items]
                      ResultColumn
                        flags: (none)
                        alias: (none)
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
              right:
                SelectStmt
                  flags: (none)
                  columns:
                    ResultColumnList [1 items]
                      ResultColumn
                        flags: (none)
                        alias: (none)
                        expr:
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

    def test_union_all(self):
        return DiffTestBlueprint(
            sql="SELECT 1 UNION ALL SELECT 2",
            out="""\
            CompoundSelect
              op: UNION_ALL
              left:
                SelectStmt
                  flags: (none)
                  columns:
                    ResultColumnList [1 items]
                      ResultColumn
                        flags: (none)
                        alias: (none)
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
              right:
                SelectStmt
                  flags: (none)
                  columns:
                    ResultColumnList [1 items]
                      ResultColumn
                        flags: (none)
                        alias: (none)
                        expr:
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

    def test_intersect(self):
        return DiffTestBlueprint(
            sql="SELECT 1 INTERSECT SELECT 2",
            out="""\
            CompoundSelect
              op: INTERSECT
              left:
                SelectStmt
                  flags: (none)
                  columns:
                    ResultColumnList [1 items]
                      ResultColumn
                        flags: (none)
                        alias: (none)
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
              right:
                SelectStmt
                  flags: (none)
                  columns:
                    ResultColumnList [1 items]
                      ResultColumn
                        flags: (none)
                        alias: (none)
                        expr:
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

    def test_except(self):
        return DiffTestBlueprint(
            sql="SELECT 1 EXCEPT SELECT 2",
            out="""\
            CompoundSelect
              op: EXCEPT
              left:
                SelectStmt
                  flags: (none)
                  columns:
                    ResultColumnList [1 items]
                      ResultColumn
                        flags: (none)
                        alias: (none)
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
              right:
                SelectStmt
                  flags: (none)
                  columns:
                    ResultColumnList [1 items]
                      ResultColumn
                        flags: (none)
                        alias: (none)
                        expr:
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


class SubqueryExpr(TestSuite):
    """Subquery expression tests."""

    def test_scalar_subquery(self):
        return DiffTestBlueprint(
            sql="SELECT (SELECT 1)",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: (none)
                    expr:
                      SubqueryExpr
                        select:
                          SelectStmt
                            flags: (none)
                            columns:
                              ResultColumnList [1 items]
                                ResultColumn
                                  flags: (none)
                                  alias: (none)
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
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_exists(self):
        return DiffTestBlueprint(
            sql="SELECT EXISTS (SELECT 1)",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: (none)
                    expr:
                      ExistsExpr
                        select:
                          SelectStmt
                            flags: (none)
                            columns:
                              ResultColumnList [1 items]
                                ResultColumn
                                  flags: (none)
                                  alias: (none)
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
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )


class InExpr(TestSuite):
    """IN expression tests."""

    def test_in_list(self):
        return DiffTestBlueprint(
            sql="SELECT 1 IN (1, 2, 3)",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: (none)
                    expr:
                      InExpr
                        negated: FALSE
                        operand:
                          Literal
                            literal_type: INTEGER
                            source: "1"
                        source:
                          ExprList [3 items]
                            Literal
                              literal_type: INTEGER
                              source: "1"
                            Literal
                              literal_type: INTEGER
                              source: "2"
                            Literal
                              literal_type: INTEGER
                              source: "3"
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_not_in_list(self):
        return DiffTestBlueprint(
            sql="SELECT 1 NOT IN (1, 2, 3)",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: (none)
                    expr:
                      InExpr
                        negated: TRUE
                        operand:
                          Literal
                            literal_type: INTEGER
                            source: "1"
                        source:
                          ExprList [3 items]
                            Literal
                              literal_type: INTEGER
                              source: "1"
                            Literal
                              literal_type: INTEGER
                              source: "2"
                            Literal
                              literal_type: INTEGER
                              source: "3"
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

    def test_in_subquery(self):
        return DiffTestBlueprint(
            sql="SELECT 1 IN (SELECT 1)",
            out="""\
            SelectStmt
              flags: (none)
              columns:
                ResultColumnList [1 items]
                  ResultColumn
                    flags: (none)
                    alias: (none)
                    expr:
                      InExpr
                        negated: FALSE
                        operand:
                          Literal
                            literal_type: INTEGER
                            source: "1"
                        source:
                          SubqueryExpr
                            select:
                              SelectStmt
                                flags: (none)
                                columns:
                                  ResultColumnList [1 items]
                                    ResultColumn
                                      flags: (none)
                                      alias: (none)
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
              from_clause: (none)
              where_clause: (none)
              groupby: (none)
              having: (none)
              orderby: (none)
              limit_clause: (none)
              window_clause: (none)
""",
        )

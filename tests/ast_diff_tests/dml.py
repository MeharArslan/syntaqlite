# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""DML (INSERT/UPDATE/DELETE) AST tests."""

from python.syntaqlite.diff_tests.testing import DiffTestBlueprint, TestSuite


class DeleteBasic(TestSuite):
    """Basic DELETE statement tests."""

    def test_simple_delete(self):
        return DiffTestBlueprint(
            sql="DELETE FROM t",
            out="""\
            DeleteStmt
              table:
                TableRef
                  table_name: "t"
                  schema: null
                  alias: null
              where_clause: (none)
              orderby: (none)
              limit_clause: (none)
""",
        )

    def test_delete_with_where(self):
        return DiffTestBlueprint(
            sql="DELETE FROM t WHERE x = 1",
            out="""\
            DeleteStmt
              table:
                TableRef
                  table_name: "t"
                  schema: null
                  alias: null
              where_clause:
                BinaryExpr
                  op: EQ
                  left:
                    ColumnRef
                      column: "x"
                      table: null
                      schema: null
                  right:
                    Literal
                      literal_type: INTEGER
                      source: "1"
              orderby: (none)
              limit_clause: (none)
""",
        )

    def test_delete_with_schema(self):
        return DiffTestBlueprint(
            sql="DELETE FROM main.t",
            out="""\
            DeleteStmt
              table:
                TableRef
                  table_name: "t"
                  schema: "main"
                  alias: null
              where_clause: (none)
              orderby: (none)
              limit_clause: (none)
""",
        )


class InsertBasic(TestSuite):
    """Basic INSERT statement tests."""

    def test_insert_values(self):
        return DiffTestBlueprint(
            sql="INSERT INTO t VALUES (1, 2, 3)",
            out="""\
            InsertStmt
              conflict_action: DEFAULT
              table:
                TableRef
                  table_name: "t"
                  schema: null
                  alias: null
              columns: (none)
              source:
                ValuesClause
                  rows:
                    ValuesRowList [1 items]
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
""",
        )

    def test_insert_with_columns(self):
        return DiffTestBlueprint(
            sql="INSERT INTO t(a, b) VALUES (1, 2)",
            out="""\
            InsertStmt
              conflict_action: DEFAULT
              table:
                TableRef
                  table_name: "t"
                  schema: null
                  alias: null
              columns:
                ExprList [2 items]
                  ColumnRef
                    column: "a"
                    table: null
                    schema: null
                  ColumnRef
                    column: "b"
                    table: null
                    schema: null
              source:
                ValuesClause
                  rows:
                    ValuesRowList [1 items]
                      ExprList [2 items]
                        Literal
                          literal_type: INTEGER
                          source: "1"
                        Literal
                          literal_type: INTEGER
                          source: "2"
""",
        )

    def test_insert_from_select(self):
        return DiffTestBlueprint(
            sql="INSERT INTO t SELECT * FROM s",
            out="""\
            InsertStmt
              conflict_action: DEFAULT
              table:
                TableRef
                  table_name: "t"
                  schema: null
                  alias: null
              columns: (none)
              source:
                SelectStmt
                  flags: (none)
                  columns:
                    ResultColumnList [1 items]
                      ResultColumn
                        flags: STAR
                        alias: null
                        expr: (none)
                  from_clause:
                    TableRef
                      table_name: "s"
                      schema: null
                      alias: null
                  where_clause: (none)
                  groupby: (none)
                  having: (none)
                  orderby: (none)
                  limit_clause: (none)
                  window_clause: (none)
""",
        )

    def test_insert_default_values(self):
        return DiffTestBlueprint(
            sql="INSERT INTO t DEFAULT VALUES",
            out="""\
            InsertStmt
              conflict_action: DEFAULT
              table:
                TableRef
                  table_name: "t"
                  schema: null
                  alias: null
              columns: (none)
              source: (none)
""",
        )


class InsertConflict(TestSuite):
    """INSERT with conflict resolution tests."""

    def test_insert_or_replace(self):
        return DiffTestBlueprint(
            sql="INSERT OR REPLACE INTO t VALUES (1)",
            out="""\
            InsertStmt
              conflict_action: REPLACE
              table:
                TableRef
                  table_name: "t"
                  schema: null
                  alias: null
              columns: (none)
              source:
                ValuesClause
                  rows:
                    ValuesRowList [1 items]
                      ExprList [1 items]
                        Literal
                          literal_type: INTEGER
                          source: "1"
""",
        )

    def test_replace_into(self):
        return DiffTestBlueprint(
            sql="REPLACE INTO t VALUES (1)",
            out="""\
            InsertStmt
              conflict_action: REPLACE
              table:
                TableRef
                  table_name: "t"
                  schema: null
                  alias: null
              columns: (none)
              source:
                ValuesClause
                  rows:
                    ValuesRowList [1 items]
                      ExprList [1 items]
                        Literal
                          literal_type: INTEGER
                          source: "1"
""",
        )

    def test_insert_or_rollback(self):
        return DiffTestBlueprint(
            sql="INSERT OR ROLLBACK INTO t VALUES (1)",
            out="""\
            InsertStmt
              conflict_action: ROLLBACK
              table:
                TableRef
                  table_name: "t"
                  schema: null
                  alias: null
              columns: (none)
              source:
                ValuesClause
                  rows:
                    ValuesRowList [1 items]
                      ExprList [1 items]
                        Literal
                          literal_type: INTEGER
                          source: "1"
""",
        )

    def test_insert_or_abort(self):
        return DiffTestBlueprint(
            sql="INSERT OR ABORT INTO t VALUES (1)",
            out="""\
            InsertStmt
              conflict_action: ABORT
              table:
                TableRef
                  table_name: "t"
                  schema: null
                  alias: null
              columns: (none)
              source:
                ValuesClause
                  rows:
                    ValuesRowList [1 items]
                      ExprList [1 items]
                        Literal
                          literal_type: INTEGER
                          source: "1"
""",
        )

    def test_insert_or_fail(self):
        return DiffTestBlueprint(
            sql="INSERT OR FAIL INTO t VALUES (1)",
            out="""\
            InsertStmt
              conflict_action: FAIL
              table:
                TableRef
                  table_name: "t"
                  schema: null
                  alias: null
              columns: (none)
              source:
                ValuesClause
                  rows:
                    ValuesRowList [1 items]
                      ExprList [1 items]
                        Literal
                          literal_type: INTEGER
                          source: "1"
""",
        )

    def test_insert_or_ignore(self):
        return DiffTestBlueprint(
            sql="INSERT OR IGNORE INTO t VALUES (1)",
            out="""\
            InsertStmt
              conflict_action: IGNORE
              table:
                TableRef
                  table_name: "t"
                  schema: null
                  alias: null
              columns: (none)
              source:
                ValuesClause
                  rows:
                    ValuesRowList [1 items]
                      ExprList [1 items]
                        Literal
                          literal_type: INTEGER
                          source: "1"
""",
        )


class UpdateBasic(TestSuite):
    """Basic UPDATE statement tests."""

    def test_simple_update(self):
        return DiffTestBlueprint(
            sql="UPDATE t SET x = 1",
            out="""\
            UpdateStmt
              conflict_action: DEFAULT
              table:
                TableRef
                  table_name: "t"
                  schema: null
                  alias: null
              setlist:
                SetClauseList [1 items]
                  SetClause
                    column: "x"
                    columns: (none)
                    value:
                      Literal
                        literal_type: INTEGER
                        source: "1"
              from_clause: (none)
              where_clause: (none)
              orderby: (none)
              limit_clause: (none)
""",
        )

    def test_update_with_where(self):
        return DiffTestBlueprint(
            sql="UPDATE t SET x = 1, y = 2 WHERE id = 3",
            out="""\
            UpdateStmt
              conflict_action: DEFAULT
              table:
                TableRef
                  table_name: "t"
                  schema: null
                  alias: null
              setlist:
                SetClauseList [2 items]
                  SetClause
                    column: "x"
                    columns: (none)
                    value:
                      Literal
                        literal_type: INTEGER
                        source: "1"
                  SetClause
                    column: "y"
                    columns: (none)
                    value:
                      Literal
                        literal_type: INTEGER
                        source: "2"
              from_clause: (none)
              where_clause:
                BinaryExpr
                  op: EQ
                  left:
                    ColumnRef
                      column: "id"
                      table: null
                      schema: null
                  right:
                    Literal
                      literal_type: INTEGER
                      source: "3"
              orderby: (none)
              limit_clause: (none)
""",
        )

    def test_update_or_ignore(self):
        return DiffTestBlueprint(
            sql="UPDATE OR IGNORE t SET x = 1",
            out="""\
            UpdateStmt
              conflict_action: IGNORE
              table:
                TableRef
                  table_name: "t"
                  schema: null
                  alias: null
              setlist:
                SetClauseList [1 items]
                  SetClause
                    column: "x"
                    columns: (none)
                    value:
                      Literal
                        literal_type: INTEGER
                        source: "1"
              from_clause: (none)
              where_clause: (none)
              orderby: (none)
              limit_clause: (none)
""",
        )


class DmlWithCte(TestSuite):
    """DML statements with CTEs."""

    def test_insert_with_cte(self):
        return DiffTestBlueprint(
            sql="WITH cte AS (SELECT 1) INSERT INTO t SELECT * FROM cte",
            out="""\
            WithClause
              recursive: FALSE
              ctes:
                CteList [1 items]
                  CteDefinition
                    cte_name: "cte"
                    materialized: DEFAULT
                    columns: (none)
                    select:
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
              select:
                InsertStmt
                  conflict_action: DEFAULT
                  table:
                    TableRef
                      table_name: "t"
                      schema: null
                      alias: null
                  columns: (none)
                  source:
                    SelectStmt
                      flags: (none)
                      columns:
                        ResultColumnList [1 items]
                          ResultColumn
                            flags: STAR
                            alias: null
                            expr: (none)
                      from_clause:
                        TableRef
                          table_name: "cte"
                          schema: null
                          alias: null
                      where_clause: (none)
                      groupby: (none)
                      having: (none)
                      orderby: (none)
                      limit_clause: (none)
                      window_clause: (none)
""",
        )

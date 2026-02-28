# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

from python.syntaqlite.diff_tests.testing import AstTestBlueprint, TestSuite


class PerfettoTableValidation(TestSuite):
    def test_create_perfetto_table_unknown_table(self):
        """Key regression test for walk_other_node Expr-before-Stmt fix.

        CREATE PERFETTO TABLE routes through walk_other_node which must
        dispatch the inner SELECT via walk_stmt (not walk_expr) so that
        FROM-clause table resolution runs before column ref checks.
        The result should be a single 'unknown table' warning, NOT an
        'unknown column' diagnostic.
        """
        return AstTestBlueprint(
            sql="CREATE PERFETTO TABLE t AS SELECT dur FROM slice",
            out="warning 43..48: unknown table 'slice'",
        )

    def test_create_perfetto_table_known_table(self):
        return AstTestBlueprint(
            sql="CREATE TABLE slice(dur INT); CREATE PERFETTO TABLE t AS SELECT dur FROM slice",
            out="",
        )


class PerfettoViewValidation(TestSuite):
    def test_create_perfetto_view_unknown_table(self):
        return AstTestBlueprint(
            sql="CREATE PERFETTO VIEW v AS SELECT dur FROM slice",
            out="warning 42..47: unknown table 'slice'",
        )


class PerfettoFunctionValidation(TestSuite):
    def test_create_perfetto_function_unknown_table(self):
        return AstTestBlueprint(
            sql="CREATE PERFETTO FUNCTION f() RETURNS INT AS SELECT dur FROM slice",
            out="warning 60..65: unknown table 'slice'",
        )


class BaselineValidation(TestSuite):
    def test_plain_select_unknown_table(self):
        return AstTestBlueprint(
            sql="SELECT dur FROM slice",
            out="warning 16..21: unknown table 'slice'",
        )

    def test_known_table_no_warnings(self):
        return AstTestBlueprint(
            sql="CREATE TABLE slice(dur INT); SELECT dur FROM slice",
            out="",
        )

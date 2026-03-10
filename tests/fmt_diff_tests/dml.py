# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

from python.syntaqlite.diff_tests.testing import DiffTestBlueprint, TestSuite


class DmlFormat(TestSuite):
    def test_delete(self):
        return DiffTestBlueprint(
            sql="delete from t where x = 1",
            out="DELETE FROM t WHERE x = 1;",
        )

    def test_delete_no_where(self):
        return DiffTestBlueprint(
            sql="delete from t",
            out="DELETE FROM t;",
        )

    def test_update(self):
        return DiffTestBlueprint(
            sql="update t set x = 1",
            out="UPDATE t SET x = 1;",
        )

    def test_update_where(self):
        return DiffTestBlueprint(
            sql="update t set x = 1 where y = 2",
            out="UPDATE t SET x = 1 WHERE y = 2;",
        )

    def test_update_or_rollback(self):
        return DiffTestBlueprint(
            sql="update or rollback t set x = 1",
            out="UPDATE OR ROLLBACK t SET x = 1;",
        )

    def test_update_or_abort(self):
        return DiffTestBlueprint(
            sql="update or abort t set x = 1",
            out="UPDATE OR ABORT t SET x = 1;",
        )

    def test_update_or_replace(self):
        return DiffTestBlueprint(
            sql="update or replace t set x = 1",
            out="UPDATE OR REPLACE t SET x = 1;",
        )

    def test_update_multiple_set(self):
        return DiffTestBlueprint(
            sql="update t set x = 1, y = 2, z = 3",
            out="UPDATE t SET x = 1, y = 2, z = 3;",
        )

    def test_insert_values(self):
        return DiffTestBlueprint(
            sql="insert into t values (1, 2)",
            out="INSERT INTO t VALUES (1, 2);",
        )

    def test_insert_columns(self):
        return DiffTestBlueprint(
            sql="insert into t (a, b) values (1, 2)",
            out="INSERT INTO t(a, b) VALUES (1, 2);",
        )

    def test_insert_space_before_values(self):
        return DiffTestBlueprint(
            sql="INSERT INTO t(a, b) VALUES(1, 2)",
            out="INSERT INTO t(a, b) VALUES (1, 2);",
        )

    def test_insert_or_replace(self):
        return DiffTestBlueprint(
            sql="insert or replace into t values (1)",
            out="REPLACE INTO t VALUES (1);",
        )

    def test_replace(self):
        return DiffTestBlueprint(
            sql="replace into t values (1)",
            out="REPLACE INTO t VALUES (1);",
        )

    def test_insert_or_ignore(self):
        return DiffTestBlueprint(
            sql="insert or ignore into t values (1)",
            out="INSERT OR IGNORE INTO t VALUES (1);",
        )

    def test_insert_default_values(self):
        return DiffTestBlueprint(
            sql="insert into t default values",
            out="INSERT INTO t DEFAULT VALUES;",
        )


class ReturningFormat(TestSuite):
    def test_delete_returning_star(self):
        return DiffTestBlueprint(
            sql="delete from t returning *",
            out="DELETE FROM t RETURNING *;",
        )

    def test_delete_where_returning(self):
        return DiffTestBlueprint(
            sql="delete from t where id = 1 returning id, name",
            out="DELETE FROM t WHERE id = 1 RETURNING id, name;",
        )

    def test_update_returning(self):
        return DiffTestBlueprint(
            sql="update t set x = 1 returning *",
            out="UPDATE t SET x = 1 RETURNING *;",
        )

    def test_update_where_returning(self):
        return DiffTestBlueprint(
            sql="update t set x = 1 where id = 1 returning id, x",
            out="UPDATE t SET x = 1 WHERE id = 1 RETURNING id, x;",
        )

    def test_insert_returning(self):
        return DiffTestBlueprint(
            sql="insert into t values (1) returning id",
            out="INSERT INTO t VALUES (1) RETURNING id;",
        )

    def test_insert_default_values_returning(self):
        return DiffTestBlueprint(
            sql="insert into t default values returning *",
            out="INSERT INTO t DEFAULT VALUES RETURNING *;",
        )


class UpsertFormat(TestSuite):
    def test_on_conflict_do_nothing(self):
        return DiffTestBlueprint(
            sql="insert into t values (1) on conflict do nothing",
            out="INSERT INTO t VALUES (1) ON CONFLICT DO NOTHING;",
        )

    def test_on_conflict_do_update(self):
        return DiffTestBlueprint(
            sql="insert into t values (1) on conflict do update set x = 1",
            out="INSERT INTO t VALUES (1) ON CONFLICT DO UPDATE SET x = 1;",
        )

    def test_on_conflict_column_do_nothing(self):
        return DiffTestBlueprint(
            sql="insert into t values (1) on conflict(id) do nothing",
            out="INSERT INTO t VALUES (1) ON CONFLICT (id) DO NOTHING;",
        )

    def test_on_conflict_column_do_update_where(self):
        return DiffTestBlueprint(
            sql="insert into t values (1) on conflict(id) do update set x = 1 where x != 1",
            out="INSERT INTO t VALUES (1) ON CONFLICT (id) DO UPDATE SET x = 1 WHERE x != 1;",
        )

    def test_on_conflict_do_nothing_returning(self):
        return DiffTestBlueprint(
            sql="insert into t values (1) on conflict do nothing returning *",
            out="INSERT INTO t VALUES (1) ON CONFLICT DO NOTHING RETURNING *;",
        )
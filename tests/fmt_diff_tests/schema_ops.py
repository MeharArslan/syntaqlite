# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

from python.syntaqlite.diff_tests.testing import DiffTestBlueprint, TestSuite


class DropFormat(TestSuite):
    def test_drop_table(self):
        return DiffTestBlueprint(
            sql="drop table t",
            out="DROP TABLE t;",
        )

    def test_drop_table_if_exists(self):
        return DiffTestBlueprint(
            sql="drop table if exists t",
            out="DROP TABLE IF EXISTS t;",
        )

    def test_drop_table_schema(self):
        return DiffTestBlueprint(
            sql="drop table main.t",
            out="DROP TABLE main.t;",
        )

    def test_drop_index(self):
        return DiffTestBlueprint(
            sql="drop index idx",
            out="DROP INDEX idx;",
        )

    def test_drop_view(self):
        return DiffTestBlueprint(
            sql="drop view v",
            out="DROP VIEW v;",
        )

    def test_drop_trigger(self):
        return DiffTestBlueprint(
            sql="drop trigger tr",
            out="DROP TRIGGER tr;",
        )


class AlterTableFormat(TestSuite):
    def test_rename_table(self):
        return DiffTestBlueprint(
            sql="alter table t rename to t2",
            out="ALTER TABLE t RENAME TO t2;",
        )

    def test_rename_column(self):
        return DiffTestBlueprint(
            sql="alter table t rename column c1 to c2",
            out="ALTER TABLE t RENAME COLUMN c1 TO c2;",
        )

    def test_drop_column(self):
        return DiffTestBlueprint(
            sql="alter table t drop column c1",
            out="ALTER TABLE t DROP COLUMN c1;",
        )

    def test_add_column(self):
        return DiffTestBlueprint(
            sql="alter table t add column c1",
            out="ALTER TABLE t ADD COLUMN c1;",
        )


class TransactionFormat(TestSuite):
    def test_begin(self):
        return DiffTestBlueprint(
            sql="begin",
            out="BEGIN;",
        )

    def test_begin_immediate(self):
        return DiffTestBlueprint(
            sql="begin immediate",
            out="BEGIN IMMEDIATE;",
        )

    def test_begin_exclusive(self):
        return DiffTestBlueprint(
            sql="begin exclusive",
            out="BEGIN EXCLUSIVE;",
        )

    def test_commit(self):
        return DiffTestBlueprint(
            sql="commit",
            out="COMMIT;",
        )

    def test_end(self):
        return DiffTestBlueprint(
            sql="end",
            out="COMMIT;",
        )

    def test_rollback(self):
        return DiffTestBlueprint(
            sql="rollback",
            out="ROLLBACK;",
        )


class SavepointFormat(TestSuite):
    def test_savepoint(self):
        return DiffTestBlueprint(
            sql="savepoint sp1",
            out="SAVEPOINT sp1;",
        )

    def test_release(self):
        return DiffTestBlueprint(
            sql="release sp1",
            out="RELEASE SAVEPOINT sp1;",
        )

    def test_release_savepoint(self):
        return DiffTestBlueprint(
            sql="release savepoint sp1",
            out="RELEASE SAVEPOINT sp1;",
        )

    def test_rollback_to(self):
        return DiffTestBlueprint(
            sql="rollback to sp1",
            out="ROLLBACK TO SAVEPOINT sp1;",
        )

    def test_rollback_to_savepoint(self):
        return DiffTestBlueprint(
            sql="rollback to savepoint sp1",
            out="ROLLBACK TO SAVEPOINT sp1;",
        )
# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Validate CLI integration test suite.

Tests the `syntaqlite validate` subcommand, focusing on --schema separation.
"""

from __future__ import annotations

import subprocess
import tempfile
from pathlib import Path

from python.dev.integration_tests.suite import SuiteContext

NAME = "validate"
DESCRIPTION = "Validate CLI tests (--schema flag, DDL/query separation)"

_GREEN = "\033[32m"
_RED = "\033[31m"
_RESET = "\033[0m"


def _pass(name: str) -> None:
    print(f"  {_GREEN}PASS{_RESET}  {name}")


def _fail(name: str, detail: str) -> None:
    print(f"  {_RED}FAIL{_RESET}  {name}: {detail}")


def _run(binary: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(binary), "validate", *args],
        capture_output=True,
        text=True,
    )


def _test_schema_flag_valid(ctx: SuiteContext) -> bool:
    """Query referencing table from --schema should produce no errors."""
    with tempfile.TemporaryDirectory() as tmp:
        schema = Path(tmp) / "schema.sql"
        query = Path(tmp) / "query.sql"
        schema.write_text("CREATE TABLE users (id INTEGER, name TEXT);\n")
        query.write_text("SELECT name FROM users;\n")

        result = _run(ctx.binary, "--schema", str(schema), str(query))
        if result.returncode != 0:
            _fail("schema_flag_valid", f"exit {result.returncode}: {result.stderr}")
            return False
        _pass("schema_flag_valid")
        return True


def _test_schema_flag_unknown_column(ctx: SuiteContext) -> bool:
    """Query referencing bad column should produce a warning."""
    with tempfile.TemporaryDirectory() as tmp:
        schema = Path(tmp) / "schema.sql"
        query = Path(tmp) / "query.sql"
        schema.write_text("CREATE TABLE users (id INTEGER, name TEXT);\n")
        query.write_text("SELECT bogus FROM users;\n")

        result = _run(ctx.binary, "--schema", str(schema), str(query))
        if "bogus" not in result.stderr:
            _fail("schema_flag_unknown_column",
                   f"expected 'bogus' in stderr, got: {result.stderr}")
            return False
        _pass("schema_flag_unknown_column")
        return True


def _test_multiple_schema_files(ctx: SuiteContext) -> bool:
    """Multiple --schema files should all contribute to the catalog."""
    with tempfile.TemporaryDirectory() as tmp:
        s1 = Path(tmp) / "schema1.sql"
        s2 = Path(tmp) / "schema2.sql"
        query = Path(tmp) / "query.sql"
        s1.write_text("CREATE TABLE users (id INTEGER, name TEXT);\n")
        s2.write_text("CREATE TABLE orders (id INTEGER, total REAL);\n")
        query.write_text("SELECT name FROM users; SELECT total FROM orders;\n")

        result = _run(
            ctx.binary, "--schema", str(s1), "--schema", str(s2), str(query),
        )
        if result.returncode != 0:
            _fail("multiple_schema_files", f"exit {result.returncode}: {result.stderr}")
            return False
        _pass("multiple_schema_files")
        return True


def _test_schema_glob(ctx: SuiteContext) -> bool:
    """--schema with a glob should expand to multiple files."""
    with tempfile.TemporaryDirectory() as tmp:
        (Path(tmp) / "a.sql").write_text(
            "CREATE TABLE users (id INTEGER, name TEXT);\n")
        (Path(tmp) / "b.sql").write_text(
            "CREATE TABLE orders (id INTEGER, total REAL);\n")
        query = Path(tmp) / "query.sql"
        query.write_text("SELECT name FROM users; SELECT total FROM orders;\n")

        result = _run(ctx.binary, "--schema", f"{tmp}/*.sql", str(query))
        # The glob matches a.sql, b.sql, and query.sql — but query.sql contains
        # only DML so it won't hurt. Both tables should be visible.
        if result.returncode != 0:
            _fail("schema_glob", f"exit {result.returncode}: {result.stderr}")
            return False
        _pass("schema_glob")
        return True


def _test_no_schema_inline_ddl(ctx: SuiteContext) -> bool:
    """Without --schema, inline DDL should still work."""
    with tempfile.TemporaryDirectory() as tmp:
        f = Path(tmp) / "all.sql"
        f.write_text(
            "CREATE TABLE t (a INTEGER);\nSELECT a FROM t;\n")

        result = _run(ctx.binary, str(f))
        if result.returncode != 0:
            _fail("no_schema_inline_ddl", f"exit {result.returncode}: {result.stderr}")
            return False
        _pass("no_schema_inline_ddl")
        return True


def _test_config_file_schema(ctx: SuiteContext) -> bool:
    """syntaqlite.toml schema should be picked up without --schema."""
    with tempfile.TemporaryDirectory() as tmp:
        schema = Path(tmp) / "schema.sql"
        query = Path(tmp) / "src" / "query.sql"
        config = Path(tmp) / "syntaqlite.toml"

        schema.write_text("CREATE TABLE users (id INTEGER, name TEXT);\n")
        query.parent.mkdir()
        query.write_text("SELECT name FROM users;\n")
        config.write_text('schema = ["schema.sql"]\n')

        result = subprocess.run(
            [str(ctx.binary), "validate", str(query)],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            _fail("config_file_schema",
                   f"exit {result.returncode}: {result.stderr}")
            return False
        _pass("config_file_schema")
        return True


def _test_config_file_glob_routing(ctx: SuiteContext) -> bool:
    """[schemas] glob routing should match files to the right schema."""
    with tempfile.TemporaryDirectory() as tmp:
        schema_a = Path(tmp) / "schema_a.sql"
        schema_b = Path(tmp) / "schema_b.sql"
        src_dir = Path(tmp) / "src"
        test_dir = Path(tmp) / "tests"
        src_dir.mkdir()
        test_dir.mkdir()

        schema_a.write_text("CREATE TABLE users (id INTEGER, name TEXT);\n")
        schema_b.write_text("CREATE TABLE fixtures (id INTEGER, data TEXT);\n")

        (src_dir / "query.sql").write_text("SELECT name FROM users;\n")
        (test_dir / "query.sql").write_text("SELECT data FROM fixtures;\n")

        config = Path(tmp) / "syntaqlite.toml"
        config.write_text(
            '[schemas]\n'
            '"src/**/*.sql" = ["schema_a.sql"]\n'
            '"tests/**/*.sql" = ["schema_b.sql"]\n'
        )

        # src/query.sql should validate against schema_a (users table).
        r1 = subprocess.run(
            [str(ctx.binary), "validate", str(src_dir / "query.sql")],
            capture_output=True,
            text=True,
        )
        if r1.returncode != 0:
            _fail("config_file_glob_routing",
                   f"src/query.sql failed: {r1.stderr}")
            return False

        # tests/query.sql should validate against schema_b (fixtures table).
        r2 = subprocess.run(
            [str(ctx.binary), "validate", str(test_dir / "query.sql")],
            capture_output=True,
            text=True,
        )
        if r2.returncode != 0:
            _fail("config_file_glob_routing",
                   f"tests/query.sql failed: {r2.stderr}")
            return False

        _pass("config_file_glob_routing")
        return True


def _test_config_file_format(ctx: SuiteContext) -> bool:
    """[format] section should apply to fmt command."""
    with tempfile.TemporaryDirectory() as tmp:
        query = Path(tmp) / "query.sql"
        config = Path(tmp) / "syntaqlite.toml"

        query.write_text("select 1;\n")
        config.write_text('[format]\nkeyword-case = "lower"\n')

        result = subprocess.run(
            [str(ctx.binary), "fmt", str(query)],
            capture_output=True,
            text=True,
            cwd=tmp,
        )
        if "select" not in result.stdout:
            _fail("config_file_format",
                   f"expected lowercase 'select', got: {result.stdout!r}")
            return False
        if "SELECT" in result.stdout:
            _fail("config_file_format",
                   f"got uppercase 'SELECT' despite keyword-case=lower: {result.stdout!r}")
            return False
        _pass("config_file_format")
        return True


def _test_config_file_cli_override(ctx: SuiteContext) -> bool:
    """CLI flags should override config file values."""
    with tempfile.TemporaryDirectory() as tmp:
        query = Path(tmp) / "query.sql"
        config = Path(tmp) / "syntaqlite.toml"

        query.write_text("select 1;\n")
        config.write_text('[format]\nkeyword-case = "lower"\n')

        # --keyword-case upper should override config's "lower".
        result = subprocess.run(
            [str(ctx.binary), "fmt", "-k", "upper", str(query)],
            capture_output=True,
            text=True,
        )
        if "SELECT" not in result.stdout:
            _fail("config_file_cli_override",
                   f"expected uppercase 'SELECT' with -k upper override, got: {result.stdout!r}")
            return False
        _pass("config_file_cli_override")
        return True


def _test_config_file_nearest_wins(ctx: SuiteContext) -> bool:
    """Innermost syntaqlite.toml should take precedence over outer one."""
    with tempfile.TemporaryDirectory() as tmp:
        # Outer config points at a schema with table "outer_t".
        outer_schema = Path(tmp) / "outer.sql"
        outer_schema.write_text("CREATE TABLE outer_t (id INTEGER);\n")
        (Path(tmp) / "syntaqlite.toml").write_text('schema = ["outer.sql"]\n')

        # Inner config points at a schema with table "inner_t".
        inner = Path(tmp) / "sub"
        inner.mkdir()
        inner_schema = inner / "inner.sql"
        inner_schema.write_text("CREATE TABLE inner_t (id INTEGER);\n")
        (inner / "syntaqlite.toml").write_text('schema = ["inner.sql"]\n')

        # Query in sub/ references inner_t — should pass with inner config.
        query = inner / "query.sql"
        query.write_text("SELECT id FROM inner_t;\n")
        r1 = subprocess.run(
            [str(ctx.binary), "validate", str(query)],
            capture_output=True,
            text=True,
        )
        if r1.returncode != 0:
            _fail("config_file_nearest_wins",
                   f"inner_t should be valid: {r1.stderr}")
            return False

        # Same query referencing outer_t — should warn (inner config doesn't
        # know about outer_t).
        query.write_text("SELECT id FROM outer_t;\n")
        r2 = subprocess.run(
            [str(ctx.binary), "validate", str(query)],
            capture_output=True,
            text=True,
        )
        if "outer_t" not in r2.stderr:
            _fail("config_file_nearest_wins",
                   f"expected 'outer_t' warning, got: {r2.stderr}")
            return False

        _pass("config_file_nearest_wins")
        return True


def _test_no_ddl_leak_across_files(ctx: SuiteContext) -> bool:
    """DDL in one query file should NOT leak to the next query file."""
    with tempfile.TemporaryDirectory() as tmp:
        f1 = Path(tmp) / "a.sql"
        f2 = Path(tmp) / "b.sql"
        f1.write_text("CREATE TABLE local_t (x INTEGER);\n")
        f2.write_text("SELECT x FROM local_t;\n")

        # Without --schema, each file gets its own analyzer, so local_t from
        # a.sql should NOT be visible in b.sql.
        result = _run(ctx.binary, str(f1), str(f2))
        if "local_t" not in result.stderr:
            _fail("no_ddl_leak_across_files",
                   f"expected 'local_t' warning in stderr, got: {result.stderr}")
            return False
        _pass("no_ddl_leak_across_files")
        return True


# ── Suite entry point ─────────────────────────────────────────────────────

def run(ctx: SuiteContext) -> int:
    tests = [
        _test_schema_flag_valid,
        _test_schema_flag_unknown_column,
        _test_multiple_schema_files,
        _test_schema_glob,
        _test_no_schema_inline_ddl,
        _test_no_ddl_leak_across_files,
        _test_config_file_schema,
        _test_config_file_glob_routing,
        _test_config_file_format,
        _test_config_file_cli_override,
        _test_config_file_nearest_wins,
    ]
    results = [t(ctx) for t in tests]
    passed = sum(results)
    total = len(results)
    print(f"\n  {passed}/{total} validate tests passed.")
    return 0 if all(results) else 1

# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Perfetto dialect formatter diff test suite."""

from python.syntaqlite.integration_tests.suite import SuiteContext

NAME = "perfetto-fmt"
DESCRIPTION = "Perfetto dialect formatter diff tests (tests/perfetto_fmt_diff_tests/)"


def run(ctx: SuiteContext) -> int:
    from python.syntaqlite.diff_tests.perfetto_common import run_perfetto_tests

    argv = [
        "--binary", str(ctx.binary),
        "--subcommand", "fmt",
        "--test-dir", "tests/perfetto_fmt_diff_tests",
    ]
    if ctx.filter_pattern:
        argv += ["--filter", ctx.filter_pattern]
    if ctx.rebaseline:
        argv.append("--rebaseline")
    if ctx.verbose >= 1:
        argv.append("-v")
    if ctx.jobs is not None:
        argv += ["--jobs", str(ctx.jobs)]
    return run_perfetto_tests(
        subcommand="fmt",
        test_dir="tests/perfetto_fmt_diff_tests",
        tempfile_prefix="syntaqlite_perfetto_fmt_",
        argv=argv,
    )

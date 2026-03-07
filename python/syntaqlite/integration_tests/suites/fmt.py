# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""SQLite formatter diff test suite."""

from python.syntaqlite.integration_tests.suite import SuiteContext

NAME = "fmt"
DESCRIPTION = "SQLite formatter diff tests (tests/fmt_diff_tests/)"


def run(ctx: SuiteContext) -> int:
    from python.syntaqlite.diff_tests.runner import main

    argv = [
        "--binary", str(ctx.binary),
        "--subcommand", "fmt",
        "--test-dir", "tests/fmt_diff_tests",
    ]
    if ctx.filter_pattern:
        argv += ["--filter", ctx.filter_pattern]
    if ctx.rebaseline:
        argv.append("--rebaseline")
    if ctx.verbose >= 1:
        argv.append("-v")
    if ctx.jobs is not None:
        argv += ["--jobs", str(ctx.jobs)]
    return main(argv)

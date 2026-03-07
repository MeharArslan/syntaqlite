# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""SQLite AST diff test suite."""

from python.syntaqlite.integration_tests.suite import SuiteContext

NAME = "ast"
DESCRIPTION = "SQLite AST diff tests (tests/ast_diff_tests/)"


def run(ctx: SuiteContext) -> int:
    from python.syntaqlite.diff_tests.runner import main

    argv = [
        "--binary", str(ctx.binary),
        "--subcommand", "ast",
        "--test-dir", "tests/ast_diff_tests",
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

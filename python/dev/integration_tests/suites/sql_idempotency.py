# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""SQL formatting idempotency test suite.

Verifies that formatting SQL preserves its AST by harvesting SQL from all
existing diff test suites and checking that parse(sql) == parse(format(sql)).
"""

from python.dev.integration_tests.suite import SuiteContext

NAME = "sql-idempotency"
DESCRIPTION = "Verify formatting preserves AST semantics (harvests SQL from ast/fmt tests)"


def run(ctx: SuiteContext) -> int:
    from python.dev.diff_tests.idempotency_runner import main

    argv = [
        "--binary", str(ctx.binary),
    ]
    if ctx.filter_pattern:
        argv += ["--filter", ctx.filter_pattern]
    if ctx.verbose >= 1:
        argv.append("-v")
    if ctx.jobs is not None:
        argv += ["--jobs", str(ctx.jobs)]
    return main(argv)

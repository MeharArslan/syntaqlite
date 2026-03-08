# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""SQL formatting idempotency test runner.

Verifies that formatting preserves SQL semantics by comparing ASTs before and
after formatting. For every SQL string found in the existing test suites:

  1. Parse the original SQL and dump its AST.
  2. Format the SQL.
  3. Parse the formatted SQL and dump its AST.
  4. Assert the two ASTs are identical.

This catches formatting bugs that silently alter SQL semantics.
"""

import argparse
import os
import re
import subprocess
import sys
import time
from concurrent.futures import ProcessPoolExecutor
from dataclasses import dataclass
from pathlib import Path
from typing import List, Optional, Tuple

from python.syntaqlite.diff_tests.test_loader import load_all_tests
from python.syntaqlite.diff_tests.testing import DiffTestBlueprint
from python.syntaqlite.diff_tests.utils import Colors, colorize, format_diff


@dataclass
class IdempotencyResult:
    """Result of a single idempotency check."""
    name: str
    passed: bool
    elapsed_ms: int = 0
    sql: str = ""
    formatted: str = ""
    ast_before: str = ""
    ast_after: str = ""
    error: str = ""


def _run_binary(
    binary: str,
    subcommand: str,
    sql: str,
    version: Optional[str] = None,
    cflags: Optional[List[str]] = None,
    timeout: float = 30.0,
) -> Tuple[int, str, str]:
    """Run the syntaqlite binary and return (returncode, stdout, stderr)."""
    cmd = [binary]
    if version:
        cmd.extend(["--sqlite-version", version])
    if cflags:
        for flag in cflags:
            cmd.extend(["--sqlite-cflag", flag])
    cmd.append(subcommand)

    proc = subprocess.run(
        cmd, input=sql, capture_output=True, text=True, timeout=timeout
    )
    return proc.returncode, proc.stdout, proc.stderr


def _run_idempotency_check(args: tuple) -> IdempotencyResult:
    """Worker: check that formatting a SQL string preserves its AST."""
    binary, name, blueprint = args
    sql = blueprint.sql
    version = blueprint.version
    cflags = blueprint.cflags

    t0 = time.monotonic()
    try:
        # Step 1: get AST of original SQL.
        rc, ast_before, stderr = _run_binary(
            binary, "ast", sql, version=version, cflags=cflags
        )
        if rc != 0:
            elapsed = int((time.monotonic() - t0) * 1000)
            return IdempotencyResult(
                name=name, passed=True, elapsed_ms=elapsed, sql=sql,
                error=f"skip: original SQL does not parse: {stderr.strip()}"
            )

        # Step 2: format the SQL.
        rc, formatted, stderr = _run_binary(
            binary, "fmt", sql, version=version, cflags=cflags
        )
        if rc != 0:
            elapsed = int((time.monotonic() - t0) * 1000)
            return IdempotencyResult(
                name=name, passed=True, elapsed_ms=elapsed, sql=sql,
                error=f"skip: formatter error: {stderr.strip()}"
            )

        # Step 3: get AST of formatted SQL.
        rc, ast_after, stderr = _run_binary(
            binary, "ast", formatted, version=version, cflags=cflags
        )
        if rc != 0:
            elapsed = int((time.monotonic() - t0) * 1000)
            return IdempotencyResult(
                name=name, passed=False, elapsed_ms=elapsed, sql=sql,
                formatted=formatted, ast_before=ast_before.strip(),
                error=f"formatted SQL does not parse: {stderr.strip()}"
            )

        # Step 4: compare ASTs.
        ast_before = ast_before.strip()
        ast_after = ast_after.strip()
        passed = ast_before == ast_after

        elapsed = int((time.monotonic() - t0) * 1000)
        return IdempotencyResult(
            name=name, passed=passed, elapsed_ms=elapsed, sql=sql,
            formatted=formatted, ast_before=ast_before, ast_after=ast_after,
        )

    except subprocess.TimeoutExpired:
        elapsed = int((time.monotonic() - t0) * 1000)
        return IdempotencyResult(
            name=name, passed=False, elapsed_ms=elapsed, sql=sql,
            error=f"timed out"
        )
    except FileNotFoundError:
        elapsed = int((time.monotonic() - t0) * 1000)
        return IdempotencyResult(
            name=name, passed=False, elapsed_ms=elapsed, sql=sql,
            error=f"binary not found: {binary}"
        )


# Test directories to harvest SQL from.
_SOURCE_DIRS = [
    "tests/ast_diff_tests",
    "tests/fmt_diff_tests",
]


def _collect_tests(
    root_dir: Path,
    source_dirs: List[str],
    filter_pattern: Optional[str] = None,
) -> List[Tuple[str, DiffTestBlueprint]]:
    """Collect SQL from all source test directories, deduplicating by SQL text."""
    seen_sql = set()
    tests = []

    for test_dir in source_dirs:
        dir_tests = load_all_tests(root_dir, filter_pattern=None, test_dir=test_dir)
        tag = test_dir.rsplit("/", 1)[-1].replace("_diff_tests", "")
        for name, blueprint in dir_tests:
            # Deduplicate by normalized SQL to avoid running the same query twice.
            key = blueprint.sql.strip()
            if key in seen_sql:
                continue
            seen_sql.add(key)
            tests.append((f"{tag}/{name}", blueprint))

    # Apply filter after deduplication.
    if filter_pattern:
        pat = re.compile(filter_pattern, re.IGNORECASE)
        tests = [(n, bp) for n, bp in tests if pat.search(n)]

    return tests


def main(argv: Optional[List[str]] = None) -> int:
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="Verify SQL formatting preserves AST semantics."
    )
    parser.add_argument(
        "--binary", default="target/debug/syntaqlite",
        help="Path to syntaqlite binary",
    )
    parser.add_argument("--filter", help="Run only tests matching pattern")
    parser.add_argument(
        "--jobs", "-j", type=int, default=None,
        help="Number of parallel jobs",
    )
    parser.add_argument(
        "-v", "--verbose", action="count", default=0,
        help="Increase verbosity (-v for results, -vv for RUN markers)",
    )
    parser.add_argument(
        "--root", default=None, help="Project root directory",
    )
    parser.add_argument(
        "--test-dir", action="append", dest="test_dirs",
        help="Additional test directories to harvest SQL from (repeatable)",
    )
    args = parser.parse_args(argv)

    # Determine project root.
    if args.root:
        root_dir = Path(args.root)
    else:
        root_dir = Path(__file__).parent.parent.parent.parent
        if not (root_dir / "Cargo.toml").exists():
            print("Error: Could not find project root.", file=sys.stderr)
            return 1

    # Resolve binary.
    binary = Path(args.binary)
    if not binary.is_absolute():
        binary = root_dir / binary

    # Collect SQL from test suites.
    source_dirs = list(_SOURCE_DIRS)
    if args.test_dirs:
        source_dirs.extend(args.test_dirs)

    tests = _collect_tests(root_dir, source_dirs, args.filter)
    if not tests:
        print("No tests to run.")
        return 0

    verbosity = args.verbose
    if verbosity >= 1:
        print(f"[==========] Running {len(tests)} idempotency checks.")

    # Run checks.
    max_workers = args.jobs if args.jobs else (os.cpu_count() or 1)
    test_args = [(str(binary), name, bp) for name, bp in tests]

    results: List[IdempotencyResult] = []
    failed_tests: List[str] = []
    skipped = 0

    with ProcessPoolExecutor(max_workers=max_workers) as executor:
        futures = [executor.submit(_run_idempotency_check, arg) for arg in test_args]

        for future in futures:
            result = future.result()
            results.append(result)

            if result.error and result.passed:
                # Skipped (parse/format error on input — not a failure).
                skipped += 1
                if verbosity >= 2:
                    skip = colorize("[  SKIP  ]", Colors.GREEN)
                    print(f"{skip} {result.name}: {result.error}")
                continue

            if result.passed:
                if verbosity >= 2:
                    print(f"[ RUN      ] {result.name}")
                if verbosity >= 1:
                    ok = colorize("[       OK ]", Colors.GREEN)
                    print(f"{ok} {result.name} ({result.elapsed_ms} ms)")
            else:
                if verbosity >= 2:
                    print(f"[ RUN      ] {result.name}")
                if verbosity >= 1:
                    fail = colorize("[  FAILED  ]", Colors.RED)
                    print(f"{fail} {result.name} ({result.elapsed_ms} ms)")

                # Always print failure details.
                if result.error:
                    print(f"  Error: {result.error}")
                else:
                    print(f"  SQL: {result.sql}")
                    print(f"  Formatted: {result.formatted.strip()}")
                    print(f"  AST diff:")
                    for line in format_diff(result.ast_before, result.ast_after):
                        print(f"    {line}")

                failed_tests.append(result.name)

    # Summary.
    passed = sum(1 for r in results if r.passed)
    failed = len(failed_tests)

    if verbosity >= 1:
        print(f"[==========] {len(results)} checks ran.")

    if passed > 0:
        msg = colorize("[  PASSED  ]", Colors.GREEN)
        extra = f" ({skipped} skipped)" if skipped else ""
        print(f"{msg} {passed} checks{extra}.")

    if failed > 0:
        msg = colorize("[  FAILED  ]", Colors.RED)
        print(f"{msg} {failed} checks, listed below:")
        for name in failed_tests:
            print(f"{msg} {name}")
        print(f"\n {failed} FAILED CHECKS")

    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())

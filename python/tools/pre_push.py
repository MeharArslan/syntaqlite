# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Pre-push verification: all checks must pass before marking work done.

Usage:
    tools/pre-push          — run all checks (headers only, output on failure)
    tools/pre-push --fix    — auto-fix what's possible, then run remaining checks
    tools/pre-push -q       — quiet: no output unless a step fails (for agents)
    tools/pre-push -v       — verbose: step headers + full command output

Build cache strategy: the --all-features clippy step uses an isolated target
directory (target/clippy-all-features) so it doesn't invalidate the cache for
subsequent steps that run with default/no-default features. RUSTFLAGS are
never changed mid-script, avoiding full-workspace cache invalidation.

All cargo invocations go through tools/cargo (hermetic toolchain) to keep
RUSTFLAGS consistent and avoid cross-tool cache invalidation.

Phases (parallelism within each phase; phases run sequentially):
  1. Format        — cargo fmt
  2. Lint + C      — clippy variants + C format/deps (parallel lanes)
  3. Build + test  — check-public-api → build CLI → unit tests (sequential)
  4. Diff tests    — all diff/integration suites (parallel)
"""

import argparse
import os
import platform
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed

ROOT_DIR = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# ANSI color codes.
_BLUE = "\033[1;34m"
_RED = "\033[1;31m"
_GREEN = "\033[1;32m"
_YELLOW = "\033[1;33m"
_RESET = "\033[0m"

_DEAD_CODE_LINTS = [
    "-D", "dead_code", "-D", "unused_imports",
    "-D", "unused_variables", "-D", "unused_mut",
]


def _cargo_path():
    return os.path.join(ROOT_DIR, "tools", "cargo")


def _tool(name):
    return os.path.join(ROOT_DIR, "tools", name)


def _cargo_cmd(*args):
    """Build a cargo command list via the hermetic wrapper."""
    return [sys.executable, _cargo_path()] + list(args)


# ---------------------------------------------------------------------------
# Step results + runners
# ---------------------------------------------------------------------------

class StepResult:
    """Captured result of a single step."""
    __slots__ = ("desc", "returncode", "output")

    def __init__(self, desc, returncode, output):
        self.desc = desc
        self.returncode = returncode
        self.output = output

    @property
    def ok(self):
        return self.returncode == 0


def _run_capturing(desc, cmd):
    """Run *cmd* capturing stdout+stderr into a StepResult."""
    try:
        proc = subprocess.run(
            cmd, cwd=ROOT_DIR,
            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
        )
        return StepResult(desc, proc.returncode, proc.stdout)
    except Exception as exc:
        return StepResult(desc, 1, str(exc))


def run_step(desc, cmd, verbosity):
    """Run a single sequential step with immediate reporting."""
    if verbosity >= 1:
        # Verbose: header + live output.
        print(f"\n{_BLUE}==> {desc}{_RESET}")
        rc = subprocess.call(cmd, cwd=ROOT_DIR)
        return StepResult(desc, rc, "")

    if verbosity >= 0:
        print(f"{_BLUE}==> {desc}{_RESET}")

    result = _run_capturing(desc, cmd)

    if not result.ok:
        if verbosity >= 0:
            print(f"{_RED}    FAILED. Output:{_RESET}")
        else:
            print(f"{_RED}==> FAILED: {desc}{_RESET}")
        if result.output:
            sys.stdout.write(result.output)
            if not result.output.endswith("\n"):
                sys.stdout.write("\n")

    return result


# ---------------------------------------------------------------------------
# Parallel execution
# ---------------------------------------------------------------------------

def _run_lane(steps):
    """Run *steps* sequentially (capturing output). Stops on first failure.

    Each element of *steps* is ``(desc, cmd)``.  Returns ``list[StepResult]``.
    """
    results = []
    for desc, cmd in steps:
        result = _run_capturing(desc, cmd)
        results.append(result)
        if not result.ok:
            break
    return results


def run_parallel_group(lanes, verbosity):
    """Run *lanes* in parallel, report results in launch order.

    Each lane is a list of ``(desc, cmd)`` tuples executed sequentially within
    the lane.  Different lanes run concurrently via threads (subprocess-bound,
    so the GIL is not a bottleneck).

    Returns ``True`` if every step in every lane succeeded.
    """
    lane_results = [None] * len(lanes)

    with ThreadPoolExecutor(max_workers=len(lanes)) as pool:
        futures = {}
        for i, lane in enumerate(lanes):
            futures[pool.submit(_run_lane, lane)] = i

        for future in as_completed(futures):
            lane_results[futures[future]] = future.result()

    # Report in launch order.
    all_ok = True
    for results in lane_results:
        if results is None:
            continue
        for r in results:
            if r.ok:
                if verbosity >= 1:
                    print(f"\n{_BLUE}==> {r.desc}{_RESET}")
                    if r.output:
                        sys.stdout.write(r.output)
                        if not r.output.endswith("\n"):
                            sys.stdout.write("\n")
                elif verbosity >= 0:
                    print(f"{_BLUE}==> {r.desc}{_RESET}")
            else:
                all_ok = False
                if verbosity >= 0:
                    print(f"{_BLUE}==> {r.desc}{_RESET}")
                    print(f"{_RED}    FAILED. Output:{_RESET}")
                else:
                    print(f"{_RED}==> FAILED: {r.desc}{_RESET}")
                if r.output:
                    sys.stdout.write(r.output)
                    if not r.output.endswith("\n"):
                        sys.stdout.write("\n")
    return all_ok


# ---------------------------------------------------------------------------
# Emscripten detection (mirrors the shell logic)
# ---------------------------------------------------------------------------

def _has_emscripten():
    sys_name = platform.system().lower()
    machine = platform.machine().lower()
    arch = "arm64" if machine in ("arm64", "aarch64") else "amd64"

    plat_dirs = {"darwin": "mac-" + arch, "linux": "linux-" + arch}
    plat_dir = plat_dirs.get(sys_name)
    if not plat_dir:
        return False
    return os.path.isdir(
        os.path.join(ROOT_DIR, "third_party", "bin", plat_dir, "emscripten"),
    )


# ---------------------------------------------------------------------------
# Main orchestrator
# ---------------------------------------------------------------------------

def _main(fix, verbosity):
    cargo = _cargo_cmd
    clippy_all_target_dir = os.path.join(ROOT_DIR, "target", "clippy-all-features")

    # ── Phase 1: Format (sequential) ──────────────────────────────────
    if fix:
        r = run_step("cargo fmt", cargo("fmt"), verbosity)
    else:
        r = run_step("cargo fmt --check", cargo("fmt", "--check"), verbosity)
    if not r.ok:
        return r.returncode

    # ── Phase 2: Lint + C checks ──────────────────────────────────────
    if fix:
        # Fix mode: two parallel lanes, then a sequential tail.
        #   Lane A — clippy --fix --all-features (isolated target dir)
        #   Lane B — format-c (fix) + check-c-deps (no cargo lock needed)
        lanes = [
            [
                ("cargo clippy --fix --all-features",
                 cargo("clippy", "--tests", "--all-features", "--all-targets",
                       "--fix", "--allow-dirty", "--allow-staged",
                       "--target-dir", clippy_all_target_dir,
                       "--", "-D", "warnings")),
            ],
            [
                ("tools/format-c", [_tool("format-c")]),
                ("tools/check-c-deps", [_tool("check-c-deps")]),
            ],
        ]
        if not run_parallel_group(lanes, verbosity):
            return 1

        # Sequential tail: re-format after clippy --fix, then dead-code.
        for desc, cmd in [
            ("cargo fmt (post-clippy-fix)", cargo("fmt")),
            ("dead code check: default features",
             cargo("clippy", "--all-targets", "--", *_DEAD_CODE_LINTS)),
            ("dead code check: no default features",
             cargo("clippy", "--all-targets", "--no-default-features",
                   "--", *_DEAD_CODE_LINTS)),
        ]:
            r = run_step(desc, cmd, verbosity)
            if not r.ok:
                return r.returncode
    else:
        # Check mode: three parallel lanes.
        #   Lane A — clippy --all-features (isolated target dir)
        #   Lane B — clippy default → clippy no-default (shared target dir)
        #   Lane C — format-c --check + check-c-deps (no cargo)
        lanes = [
            [
                ("cargo clippy --all-features",
                 cargo("clippy", "--tests", "--all-features", "--all-targets",
                       "--target-dir", clippy_all_target_dir,
                       "--", "-D", "warnings")),
            ],
            [
                ("dead code check: default features",
                 cargo("clippy", "--all-targets", "--", *_DEAD_CODE_LINTS)),
                ("dead code check: no default features",
                 cargo("clippy", "--all-targets", "--no-default-features",
                       "--", *_DEAD_CODE_LINTS)),
            ],
            [
                ("tools/format-c --check", [_tool("format-c"), "--check"]),
                ("tools/check-c-deps", [_tool("check-c-deps")]),
            ],
        ]
        if not run_parallel_group(lanes, verbosity):
            return 1

    # ── Phase 3: API + build + unit tests (sequential) ────────────────
    # All three use target/ — must be sequential to avoid cargo lock
    # contention.
    for desc, cmd in [
        ("tools/check-public-api", [_tool("check-public-api")]),
        ("cargo build -p syntaqlite-cli", cargo("build", "-p", "syntaqlite-cli")),
        ("tools/run-unit-tests", [_tool("run-unit-tests")]),
    ]:
        r = run_step(desc, cmd, verbosity)
        if not r.ok:
            return r.returncode

    # ── Phase 4: Diff/integration tests (parallel) ────────────────────
    # Each suite uses isolated temp directories and only reads the CLI
    # binary — safe to run concurrently.
    lanes = [
        [("tools/run-ast-diff-tests", [_tool("run-ast-diff-tests")])],
        [("tools/run-fmt-diff-tests", [_tool("run-fmt-diff-tests")])],
        [("tools/run-amalg-tests", [_tool("run-amalg-tests")])],
        [("tools/run-perfetto-fmt-diff-tests",
          [_tool("run-perfetto-fmt-diff-tests")])],
        [("tools/run-perfetto-validation-diff-tests",
          [_tool("run-perfetto-validation-diff-tests")])],
    ]
    if _has_emscripten():
        lanes.append(
            [("tools/build-web-playground", [_tool("build-web-playground")])],
        )
    elif verbosity >= 0:
        print(
            f"\n{_YELLOW}==> tools/build-web-playground "
            f"(skipped — emscripten not installed){_RESET}",
        )

    if not run_parallel_group(lanes, verbosity):
        return 1

    print(f"\n{_GREEN}All pre-push checks passed.{_RESET}")
    return 0


def main():
    parser = argparse.ArgumentParser(
        description="Pre-push verification: all checks must pass.",
    )
    parser.add_argument(
        "--fix", action="store_true",
        help="Auto-fix what's possible, then run remaining checks",
    )
    group = parser.add_mutually_exclusive_group()
    group.add_argument(
        "-q", "--quiet", action="store_true",
        help="No output unless a step fails (for agents)",
    )
    group.add_argument(
        "-v", "--verbose", action="store_true",
        help="Step headers + full command output",
    )
    args = parser.parse_args()

    verbosity = -1 if args.quiet else (1 if args.verbose else 0)

    try:
        return _main(args.fix, verbosity)
    except KeyboardInterrupt:
        print(f"\n{_RED}Interrupted.{_RESET}", file=sys.stderr)
        return 130


if __name__ == "__main__":
    sys.exit(main())

# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Upstream SQLite test suite.

Runs SQLite's ~1,390 upstream TCL test files through both real SQLite
(sqlite3_prepare_v2) and syntaqlite's parser/validator side-by-side,
comparing results to detect regressions.

Disabled by default — run explicitly with:
    tools/run-integration-tests --suite upstream-sqlite
    tools/run-upstream-tests              # convenience wrapper

Prerequisites:
    - tclsh and tcl-dev installed (e.g., apt install tcl-dev)
    - SQLite sources present (run tools/install-build-deps first)
"""

from __future__ import annotations

import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile
from concurrent.futures import ProcessPoolExecutor, as_completed
from dataclasses import dataclass, field
from pathlib import Path

from python.syntaqlite.integration_tests.suite import SuiteContext

NAME = "upstream-sqlite"
DESCRIPTION = "Upstream SQLite test files through syntaqlite parser/validator"
ENABLED_BY_DEFAULT = False
NEEDS_BINARY = False


@dataclass
class Summary:
    total: int = 0
    parse_ok: int = 0
    parse_error: int = 0
    both_accept: int = 0
    both_reject: int = 0
    false_positive: int = 0
    gap: int = 0


@dataclass
class FileResult:
    file: str
    entries: list[dict] = field(default_factory=list)
    error: str | None = None


def _find_tcl_include() -> str | None:
    """Find tcl.h include directory."""
    if platform.system() == "Darwin":
        # Try Homebrew tcl-tk package first (includes tcl9 / tcl8.6).
        brew = shutil.which("brew")
        if brew:
            result = subprocess.run(
                ["brew", "--prefix", "tcl-tk"], capture_output=True, text=True,
            )
            if result.returncode == 0:
                prefix = Path(result.stdout.strip())
                # Homebrew tcl-tk 9.x puts headers under include/tcl-tk/.
                for sub in ["include/tcl-tk", "include"]:
                    p = prefix / sub
                    if (p / "tcl.h").exists():
                        return str(p)
        # Xcode SDK fallback.
        sdk = subprocess.run(
            ["xcrun", "--show-sdk-path"], capture_output=True, text=True,
        )
        if sdk.returncode == 0:
            p = Path(sdk.stdout.strip()) / "usr" / "include"
            if (p / "tcl.h").exists():
                return str(p)
        # Common paths.
        for d in ["/opt/homebrew/include", "/usr/local/include", "/usr/include"]:
            if Path(d, "tcl.h").exists():
                return d
    else:
        for d in ["/usr/include/tcl8.6", "/usr/include/tcl", "/usr/include"]:
            if Path(d, "tcl.h").exists():
                return d
    return None


def _find_tcl_lib_flags() -> list[str]:
    """Return linker flags for tcl."""
    if platform.system() == "Darwin":
        brew = shutil.which("brew")
        if brew:
            result = subprocess.run(
                ["brew", "--prefix", "tcl-tk"], capture_output=True, text=True,
            )
            if result.returncode == 0:
                lib_dir = Path(result.stdout.strip()) / "lib"
                if lib_dir.exists():
                    # Detect tcl9 vs tcl8.6.
                    for name in ["tcl9.0", "tclstub9.0", "tcl8.6"]:
                        if list(lib_dir.glob(f"lib{name}*")):
                            return [f"-L{lib_dir}", f"-l{name}"]
        return ["-ltcl8.6"]
    return ["-ltcl8.6"]


def _build_extension(ctx: SuiteContext, verbose: bool) -> Path | None:
    """Build the tclsyntaqlite TCL extension. Returns the .so/.dylib path."""
    root = ctx.root_dir
    upstream_dir = root / "upstream-tests"
    csrc = upstream_dir / "csrc" / "tclsyntaqlite.c"

    if not csrc.exists():
        print(f"  error: {csrc} not found", file=sys.stderr)
        return None

    tcl_include = _find_tcl_include()
    if not tcl_include:
        print("  error: tcl.h not found. Install tcl-dev.", file=sys.stderr)
        return None

    # Build syntaqlite as a static library (staticlib for C FFI linking).
    print("  Building syntaqlite static lib...", end=" ", flush=True)
    proc = subprocess.run(
        ["cargo", "build", "-p", "syntaqlite", "--release"],
        cwd=root, capture_output=True, text=True,
    )
    if proc.returncode != 0:
        print("FAILED")
        print(proc.stderr, file=sys.stderr)
        return None
    print("OK")

    static_lib = root / "target" / "release" / "libsyntaqlite.a"
    if not static_lib.exists():
        print(f"  error: {static_lib} not found", file=sys.stderr)
        return None

    ext = ".dylib" if platform.system() == "Darwin" else ".so"
    output = root / "target" / f"tclsyntaqlite{ext}"

    syntax_include = root / "syntaqlite-syntax" / "include"
    sqlite_amalg = root / "third_party" / "src" / "sqlite-amalgamation"

    tcl_lib_flags = _find_tcl_lib_flags()

    print("  Compiling tclsyntaqlite extension...", end=" ", flush=True)

    cc_cmd = [
        "cc", "-shared", "-fPIC", "-o", str(output),
        str(csrc),
        str(sqlite_amalg / "sqlite3.c"),
        f"-I{tcl_include}",
        f"-I{syntax_include}",
        f"-I{sqlite_amalg}",
        f"-L{static_lib.parent}",
        "-lsyntaqlite",
        *tcl_lib_flags,
        "-lpthread", "-ldl", "-lm",
        "-O2",
    ]

    if platform.system() == "Darwin":
        # macOS needs -undefined dynamic_lookup for Tcl symbols.
        cc_cmd.insert(3, "-undefined")
        cc_cmd.insert(4, "dynamic_lookup")
        # Remove -ldl (not needed on macOS).
        cc_cmd = [f for f in cc_cmd if f != "-ldl"]

    proc = subprocess.run(cc_cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        print("FAILED")
        if verbose:
            print(" ".join(cc_cmd))
        print(proc.stderr, file=sys.stderr)
        return None

    print("OK")
    return output


def _discover_test_files(test_dir: Path, filter_pat: str | None) -> list[Path]:
    """Find .test files, optionally filtered by substring."""
    files = sorted(p for p in test_dir.iterdir() if p.suffix == ".test")
    if filter_pat:
        files = [f for f in files if filter_pat in f.stem]
    return files


def _run_single_test(
    extension_lib: Path,
    tester_shim: Path,
    validate: bool,
    test_file: Path,
) -> FileResult:
    """Run one .test file through tclsh and collect JSON log entries."""
    name = test_file.name

    with tempfile.NamedTemporaryFile(
        prefix=f"syntaqlite_test_{name}_", suffix=".jsonl", delete=False,
    ) as tmp:
        log_file = Path(tmp.name)

    # Test files do `set testdir [file dirname $argv0]` then
    # `source $testdir/tester.tcl`.  Set argv0 so dirname resolves to
    # our shim directory, which contains our tester.tcl replacement.
    shim_dir = tester_shim.parent
    script = (
        f"load {extension_lib} Tclsyntaqlite\n"
        f"set argv0 {shim_dir}/test.tcl\n"
        f"source {test_file}\n"
        "syntaqlite_summary\n"
    )

    env = os.environ.copy()
    env["SYNTAQLITE_TEST_LOG"] = str(log_file)
    env["SYNTAQLITE_TEST_VALIDATE"] = "1" if validate else "0"
    env["tcl_interactive"] = "0"

    try:
        proc = subprocess.run(
            ["tclsh", "-"],
            input=script, capture_output=True, text=True,
            env=env, timeout=60,
        )
    except subprocess.TimeoutExpired:
        log_file.unlink(missing_ok=True)
        return FileResult(file=name, error=f"tclsh timed out for {name}")
    except FileNotFoundError:
        log_file.unlink(missing_ok=True)
        return FileResult(file=name, error="tclsh not found")

    entries = []
    if log_file.exists():
        try:
            for line in log_file.read_text().splitlines():
                line = line.strip()
                if not line:
                    continue
                try:
                    entries.append(json.loads(line))
                except json.JSONDecodeError:
                    pass
        finally:
            log_file.unlink(missing_ok=True)
    else:
        log_file.unlink(missing_ok=True)

    error = None
    if proc.returncode != 0:
        error = f"tclsh exited {proc.returncode}: {proc.stderr[:500]}"

    return FileResult(file=name, entries=entries, error=error)


@dataclass
class FalsePositive:
    file: str
    sql: str
    parse_error: str


def _aggregate(results: list[FileResult]) -> tuple[Summary, list[FalsePositive]]:
    """Compute summary statistics and collect false positives."""
    s = Summary()
    fps: list[FalsePositive] = []
    for fr in results:
        for entry in fr.entries:
            s.total += 1
            parse_ok = entry.get("parse_ok", False)
            sqlite_ok = entry.get("sqlite_ok", False)

            if parse_ok:
                s.parse_ok += 1
            else:
                s.parse_error += 1

            diagnostics = entry.get("diagnostics") or []
            syntaqlite_ok = parse_ok and len(diagnostics) == 0

            if sqlite_ok and syntaqlite_ok:
                s.both_accept += 1
            elif not sqlite_ok and not syntaqlite_ok:
                s.both_reject += 1
            elif sqlite_ok and not syntaqlite_ok:
                s.false_positive += 1
                fps.append(FalsePositive(
                    file=fr.file,
                    sql=entry.get("sql", ""),
                    parse_error=entry.get("parse_error", ""),
                ))
            else:
                s.gap += 1

    return s, fps


def _check_baseline(
    baseline_path: Path, summary: Summary, rebaseline: bool,
) -> int:
    """Compare against baseline. Returns number of regressions (0 = pass)."""
    data = {
        "total": summary.total,
        "parse_ok": summary.parse_ok,
        "parse_error": summary.parse_error,
        "both_accept": summary.both_accept,
        "both_reject": summary.both_reject,
        "false_positive": summary.false_positive,
        "gap": summary.gap,
    }

    if rebaseline:
        baseline_path.parent.mkdir(parents=True, exist_ok=True)
        baseline_path.write_text(json.dumps(data, indent=2) + "\n")
        print(f"\n  Baseline written to {baseline_path}")
        return 0

    if not baseline_path.exists():
        baseline_path.parent.mkdir(parents=True, exist_ok=True)
        baseline_path.write_text(json.dumps(data, indent=2) + "\n")
        print(f"\n  No baseline found. Created initial baseline at {baseline_path}")
        return 0

    old = json.loads(baseline_path.read_text())
    regressions = 0

    if summary.false_positive > old.get("false_positive", 0):
        print(
            f"  Regression: false_positive increased from "
            f"{old['false_positive']} to {summary.false_positive}",
        )
        regressions += summary.false_positive - old["false_positive"]

    if summary.parse_ok < old.get("parse_ok", 0):
        print(
            f"  Regression: parse_ok decreased from "
            f"{old['parse_ok']} to {summary.parse_ok}",
        )
        regressions += old["parse_ok"] - summary.parse_ok

    if regressions == 0:
        print("\n  No regressions from baseline.")

    return regressions


def run(ctx: SuiteContext) -> int:
    verbose = ctx.verbose >= 1
    root = ctx.root_dir

    # Check for tclsh.
    if not shutil.which("tclsh"):
        print("error: tclsh not found. Install tcl (e.g., apt install tcl).", file=sys.stderr)
        return 1

    # Check for SQLite test directory.
    test_dir = root / "third_party" / "src" / "sqlite" / "test"
    if not test_dir.is_dir():
        print(f"error: SQLite test directory not found at {test_dir}", file=sys.stderr)
        print("Run tools/install-build-deps first.", file=sys.stderr)
        return 1

    tester_shim = root / "upstream-tests" / "tcl" / "tester.tcl"
    if not tester_shim.exists():
        print(f"error: tester shim not found at {tester_shim}", file=sys.stderr)
        return 1

    # Build the TCL extension.
    extension_lib = _build_extension(ctx, verbose)
    if extension_lib is None:
        return 1

    # Discover test files.
    filter_pat = ctx.filter_pattern
    test_files = _discover_test_files(test_dir, filter_pat)
    if not test_files:
        print(f"error: No .test files found in {test_dir}", file=sys.stderr)
        return 1

    jobs = ctx.jobs or os.cpu_count() or 1
    print(f"  Found {len(test_files)} test files (jobs={jobs})")

    # Run tests.
    validate = os.environ.get("UPSTREAM_VALIDATE") == "1"
    file_results: list[FileResult] = []
    done = 0
    total = len(test_files)

    if jobs == 1:
        for test_file in test_files:
            done += 1
            print(f"\r  [{done}/{total}] {test_file.name}...", end="", flush=True)
            file_results.append(
                _run_single_test(extension_lib, tester_shim, validate, test_file),
            )
    else:
        with ProcessPoolExecutor(max_workers=jobs) as pool:
            futures = {
                pool.submit(
                    _run_single_test, extension_lib, tester_shim, validate, tf,
                ): tf
                for tf in test_files
            }
            for future in as_completed(futures):
                done += 1
                tf = futures[future]
                print(f"\r  [{done}/{total}] {tf.name}...", end="", flush=True)
                file_results.append(future.result())

    print()  # Clear progress line.

    # Aggregate and print summary.
    summary, false_positives = _aggregate(file_results)
    error_count = sum(1 for r in file_results if r.error)

    print()
    print("  === Upstream Test Summary ===")
    print(f"  Files run:            {len(file_results)}")
    print(f"  Files with errors:    {error_count}")
    print()
    print(f"  Total SQL statements: {summary.total}")
    print(f"    Parse OK:           {summary.parse_ok}")
    print(f"    Parse error:        {summary.parse_error}")
    print()
    print(f"    Both accept:        {summary.both_accept} (agreement)")
    print(f"    Both reject:        {summary.both_reject} (agreement)")
    print(f"    False positives:    {summary.false_positive} (syntaqlite rejects valid SQL)")
    print(f"    Gaps:               {summary.gap} (syntaqlite misses prepare-time error)")

    # Print false positive details.
    if false_positives:
        print()
        print("  === False Positives (syntaqlite rejects valid SQL) ===")
        for fp in false_positives:
            sql_display = fp.sql[:200]
            if len(fp.sql) > 200:
                sql_display += "..."
            print(f"    {fp.file}: {fp.parse_error}")
            print(f"      SQL: {sql_display}")
            print()

    # Baseline comparison.
    baseline_path = root / "tests" / "upstream_baselines" / "parse_acceptance.json"
    regressions = _check_baseline(baseline_path, summary, ctx.rebaseline)
    if regressions > 0:
        print(f"\n  {regressions} regression(s) detected!")
        return 1

    return 0

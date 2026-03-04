#!/usr/bin/env python3
# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Build and run syntaqlite-buildtools to generate parser and tokenizer.

Multi-stage bootstrap pipeline:
  Stage 1  (--extract): Extract C fragments from raw SQLite source.
  Stage 1b (always):    Generate functions catalog and ast_traits Rust modules.
  Stage 2  (always):    Generate base syntaqlite crate C + Rust code.

The bootstrap tool (syntaqlite-buildtools) has no dependency on any generated
files, so it can be built from a completely clean checkout.

Usage:
    python3 python/tools/run_codegen.py              # Stage 1b + 2
    python3 python/tools/run_codegen.py --extract     # Stage 1 + 1b + 2
    tools/run-codegen                             # Stage 1b + 2
    tools/run-codegen --extract                   # Stage 1 + 1b + 2
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Build and run syntaqlite-buildtools to generate parser and tokenizer."
    )
    parser.add_argument(
        "--extract",
        action="store_true",
        help="Run Stage 1: extract C fragments from raw SQLite source",
    )
    args = parser.parse_args()

    project_root = Path(__file__).parent.parent.parent
    sqlite_src = project_root / "third_party" / "src" / "sqlite"
    dialect_crate = project_root / "syntaqlite"
    actions_dir = dialect_crate / "parser-actions"
    nodes_dir = dialect_crate / "parser-nodes"
    dialect_crate_dir = project_root / "syntaqlite-syntax"
    shared_crate_dir = project_root / "syntaqlite-syntax"
    vendored_dir = project_root / "syntaqlite-buildtools" / "sqlite-vendored"

    do_extract = args.extract

    # Validate input files
    if not actions_dir.is_dir():
        print(f"Error: Parser actions directory not found at {actions_dir}", file=sys.stderr)
        return 1

    if do_extract:
        if not sqlite_src.is_dir():
            print(f"Error: SQLite source not found at {sqlite_src}", file=sys.stderr)
            return 1

        # Stage 1: Extract SQLite fragments, vendor sources, generate base_files_tables.rs
        print("Stage 1: Extracting SQLite fragments and vendoring sources...")
        result = subprocess.run(
            [
                "cargo", "run", "--release", "-p", "syntaqlite-buildtools",
                "--",
                "sqlite-extract",
                "--sqlite-src", str(sqlite_src),
                "--output-dir", str(vendored_dir),
                "--actions-dir", str(actions_dir),
                "--nodes-dir", str(nodes_dir),
            ],
            cwd=project_root,
        )
        if result.returncode != 0:
            print("Stage 1 extraction failed", file=sys.stderr)
            return result.returncode

    # Build the bootstrap tool for stages 1b and 2.
    # syntaqlite-buildtools has no dependency on any generated files, so this
    # works even from a completely clean checkout.
    print("Building syntaqlite-buildtools...")
    result = subprocess.run(
        ["cargo", "build", "--release", "-p", "syntaqlite-buildtools"],
        cwd=project_root,
    )
    if result.returncode != 0:
        print("Build failed", file=sys.stderr)
        return result.returncode

    tools_bin = project_root / "target" / "release" / "syntaqlite-buildtools"

    # Stage 1b: Generate functions catalog and ast_traits from synq files.
    # Output paths are hardcoded in the Rust binary.
    functions_json = vendored_dir / "data" / "functions.json"
    cflag_audit_json = vendored_dir / "data" / "version_cflags.json"
    cflag_versions_out = dialect_crate_dir / "src" / "sqlite" / "cflags.rs"

    print("Stage 1b: Generating functions catalog and ast_traits...")
    result = subprocess.run(
        [
            str(tools_bin),
            "codegen-sqlite-parser",
            "--functions-json", str(functions_json),
            "--actions-dir", str(actions_dir),
            "--nodes-dir", str(nodes_dir),
            "--cflag-audit-json", str(cflag_audit_json),
            "--cflag-versions-out", str(cflag_versions_out),
        ],
        cwd=project_root,
    )
    if result.returncode != 0:
        print("Stage 1b codegen-sqlite-parser failed", file=sys.stderr)
        return result.returncode

    # Stage 2: Generate base SQLite dialect C + Rust code.
    print("Stage 2: Generating base SQLite dialect...")
    result = subprocess.run(
        [
            str(tools_bin),
            "codegen-sqlite",
            "--actions-dir", str(actions_dir),
            "--nodes-dir", str(nodes_dir),
        ],
    )

    if result.returncode != 0:
        print("Codegen failed", file=sys.stderr)
        return result.returncode

    # Format generated C code
    format_c = project_root / "tools" / "format-c"
    result = subprocess.run([str(format_c)], cwd=project_root)
    if result.returncode != 0:
        print("format-c failed", file=sys.stderr)
        return result.returncode

    # Format generated Rust code
    result = subprocess.run(
        ["cargo", "fmt", "--all"],
        cwd=project_root,
    )
    if result.returncode != 0:
        print("cargo fmt failed", file=sys.stderr)
        return result.returncode

    return 0


if __name__ == "__main__":
    sys.exit(main())

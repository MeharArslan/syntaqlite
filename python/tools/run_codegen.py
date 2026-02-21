#!/usr/bin/env python3
# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Build and run syntaqlite-codegen to generate parser and tokenizer.

This script automatically determines the correct paths based on project structure
and runs the codegen tool.

Usage:
    python3 python/tools/run_codegen.py
    tools/dev/run-codegen
"""

import subprocess
import sys
from pathlib import Path


def main():
    # Automatically determine paths from project structure
    project_root = Path(__file__).parent.parent.parent
    codegen_crate = project_root / "syntaqlite-codegen"
    sqlite_src = project_root / "third_party" / "src" / "sqlite" / "src"
    dialect_crate = project_root / "syntaqlite"
    actions_dir = dialect_crate / "parser-actions"
    nodes_dir = dialect_crate / "parser-nodes"
    tokenize_c = sqlite_src / "tokenize.c"
    output_dir = project_root / "syntaqlite" / "csrc"

    # Validate input files
    if not actions_dir.is_dir():
        print(f"Error: Parser actions directory not found at {actions_dir}", file=sys.stderr)
        return 1

    if not tokenize_c.exists():
        print(f"Error: SQLite tokenizer not found at {tokenize_c}", file=sys.stderr)
        print("Please ensure third_party/src/sqlite is populated", file=sys.stderr)
        return 1

    # Build codegen
    result = subprocess.run(
        ["cargo", "build", "--release"],
        cwd=codegen_crate,
    )
    if result.returncode != 0:
        print("Build failed", file=sys.stderr)
        return result.returncode

    # Run codegen with auto-detected paths
    codegen_bin = project_root / "target" / "release" / "syntaqlite-codegen"
    result = subprocess.run(
        [
            str(codegen_bin),
            "codegen",
            "--actions-dir", str(actions_dir),
            "--nodes-dir", str(nodes_dir),
            "--output-dir", str(output_dir),
        ],
    )

    if result.returncode != 0:
        print("Codegen failed", file=sys.stderr)
        return result.returncode

    return 0


if __name__ == "__main__":
    sys.exit(main())

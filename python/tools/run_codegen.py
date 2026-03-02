#!/usr/bin/env python3
# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Build and run syntaqlite-cli codegen to generate parser and tokenizer.

Multi-stage bootstrap pipeline:
  Stage 1  (--extract): Extract C fragments from raw SQLite source.
  Stage 1b (always):    Generate functions catalog Rust module from functions.json.
  Stage 2  (always):    Generate base syntaqlite crate C + Rust code.

Usage:
    python3 python/tools/run_codegen.py              # Stage 1b + 2
    python3 python/tools/run_codegen.py --extract     # Stage 1 + 1b + 2
    tools/run-codegen                             # Stage 1b + 2
    tools/run-codegen --extract                   # Stage 1 + 1b + 2
"""

import argparse
import subprocess
import sys
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(
        description="Build and run syntaqlite-cli codegen to generate parser and tokenizer."
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
    dialect_crate_dir = project_root / "syntaqlite-parser-sqlite"
    shared_crate_dir = project_root / "syntaqlite-parser"
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
                "cargo", "run", "--release", "-p", "syntaqlite-cli",
                "--no-default-features", "--features", "sqlite-extract",
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

    # Stage 1b: Generate functions catalog from extracted functions.json.
    # Uses sqlite-extract feature (no runtime dependency) so the syntaqlite
    # crate doesn't need to compile yet — avoids the bootstrap cycle where
    # syntaqlite/src/sqlite/functions.rs depends on the generated file.
    functions_json = vendored_dir / "data" / "functions.json"
    functions_catalog_rs = rust_crate_src_dir / "src" / "functions_catalog.rs"

    print("Stage 1b: Generating functions catalog...")
    result = subprocess.run(
        [
            "cargo", "run", "--release", "-p", "syntaqlite-cli",
            "--no-default-features", "--features", "sqlite-extract",
            "--",
            "generate-functions-catalog",
            "--functions-json", str(functions_json),
            "--output", str(functions_catalog_rs),
        ],
        cwd=project_root,
    )
    if result.returncode != 0:
        print("Functions catalog generation failed", file=sys.stderr)
        return result.returncode

    # Stage 2: Build CLI with codegen-sqlite feature and run full codegen
    print("Stage 2: Generating base SQLite dialect...")
    result = subprocess.run(
        ["cargo", "build", "--release", "-p", "syntaqlite-cli", "--no-default-features", "--features", "codegen-sqlite"],
        cwd=project_root,
    )
    if result.returncode != 0:
        print("Build failed", file=sys.stderr)
        return result.returncode

    cli_bin = project_root / "target" / "release" / "syntaqlite"
    wrappers_out = dialect_crate / "src" / "sqlite" / "wrappers.rs"
    result = subprocess.run(
        [
            str(cli_bin),
            "codegen",
            "--actions-dir", str(actions_dir),
            "--nodes-dir", str(nodes_dir),
            "--dialect-crate", str(dialect_crate_dir),
            "--shared-crate", str(shared_crate_dir),
            "--wrappers-out", str(wrappers_out),
        ],
    )

    if result.returncode != 0:
        print("Codegen failed", file=sys.stderr)
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

#!/usr/bin/env python3
# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Autoformat all project C source files using clang-format.

Formats .c and .h files under syntaqlite-parser/ and syntaqlite-parser-sqlite/,
skipping third_party/ and any other non-project directories.

Usage:
    python3 python/tools/format_c.py             # format in-place
    python3 python/tools/format_c.py --check     # check only (exit 1 if unformatted)
    tools/format-c                                # format in-place
    tools/format-c --check                        # check only
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT_DIR: Path = Path(__file__).parent.parent.parent
sys.path.insert(0, str(ROOT_DIR))

from python.tools.run_rust_binary import get_platform_dir

# Directories containing project-owned C code (relative to ROOT_DIR).
C_SOURCE_DIRS: list[str] = [
    "syntaqlite-parser/csrc",
    "syntaqlite-parser/include",
    "syntaqlite-parser-sqlite/csrc",
    "syntaqlite-parser-sqlite/include",
]


def find_clang_format() -> str:
    """Locate clang-format, preferring the vendored copy in third_party/bin/."""
    os_dir, ext = get_platform_dir()
    if os_dir:
        vendored = ROOT_DIR / "third_party" / "bin" / os_dir / ("clang-format" + ext)
        if vendored.is_file() and os.access(vendored, os.X_OK):
            return str(vendored)

    # Fall back to PATH (e.g. system-installed clang-format).
    system = shutil.which("clang-format")
    if system:
        return system

    print(
        "Error: clang-format not found.\n"
        "Run 'tools/install-build-deps' to install it, or install it system-wide.",
        file=sys.stderr,
    )
    sys.exit(1)


def collect_files() -> list[Path]:
    """Collect all .c and .h files under the project C source directories."""
    files: list[Path] = []
    for rel_dir in C_SOURCE_DIRS:
        src_dir = ROOT_DIR / rel_dir
        if not src_dir.is_dir():
            continue
        for path in sorted(src_dir.rglob("*.[ch]")):
            files.append(path)
    return files


def main() -> int:
    parser = argparse.ArgumentParser(description="Format project C code with clang-format.")
    parser.add_argument(
        "--check",
        action="store_true",
        help="Check formatting without modifying files (exit 1 if changes needed).",
    )
    args = parser.parse_args()

    clang_format = find_clang_format()
    files = collect_files()

    if not files:
        print("No C files found to format.")
        return 0

    cmd = [clang_format]
    if args.check:
        cmd += ["--dry-run", "--Werror"]
    else:
        cmd += ["-i"]
    cmd += [str(f) for f in files]

    print(f"{'Checking' if args.check else 'Formatting'} {len(files)} file(s)...")
    result = subprocess.run(cmd, cwd=ROOT_DIR)
    if result.returncode != 0:
        if args.check:
            print("Some files need formatting. Run 'tools/format-c' to fix.", file=sys.stderr)
        return result.returncode

    if not args.check:
        print("Done.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

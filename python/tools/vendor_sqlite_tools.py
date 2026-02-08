# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Vendor SQLite tool sources into syntaqlite-codegen crate.

This script copies SQLite's lemon and mkkeywordhash source files from
third_party/src/sqlite/ into the syntaqlite-codegen crate directory,
making the crate self-contained for publishing to crates.io.

Usage:
    python3 python/tools/vendor_sqlite_tools.py
"""

import argparse
import os
import shutil
import sys
from pathlib import Path

ROOT_DIR = Path(__file__).parent.parent.parent
SQLITE_SRC_DIR = ROOT_DIR / "third_party" / "src" / "sqlite"
CODEGEN_CRATE_DIR = ROOT_DIR / "crates" / "syntaqlite-codegen"
VENDOR_DIR = CODEGEN_CRATE_DIR / "sqlite"

# Files to vendor from SQLite
SQLITE_TOOLS = [
    ("tool/lemon.c", "lemon.c"),
    ("tool/lempar.c", "lempar.c"),
    ("tool/mkkeywordhash.c", "mkkeywordhash.c"),
]


def vendor_files(verbose=False):
    """Copy SQLite tool files into the vendor directory."""

    # Verify source directory exists
    if not SQLITE_SRC_DIR.exists():
        print(f"Error: SQLite source not found at {SQLITE_SRC_DIR}", file=sys.stderr)
        print("Run: python3 python/tools/install_build_deps.py", file=sys.stderr)
        return False

    # Create vendor directory
    VENDOR_DIR.mkdir(parents=True, exist_ok=True)
    if verbose:
        print(f"Created vendor directory: {VENDOR_DIR}")

    # Copy each file
    for src_rel, dest_name in SQLITE_TOOLS:
        src_path = SQLITE_SRC_DIR / src_rel
        dest_path = VENDOR_DIR / dest_name

        if not src_path.exists():
            print(f"Error: Source file not found: {src_path}", file=sys.stderr)
            return False

        shutil.copy2(src_path, dest_path)
        if verbose:
            print(f"Copied: {src_rel} -> {dest_path.relative_to(ROOT_DIR)}")

    # Write a README to explain the vendored files
    readme_path = VENDOR_DIR / "README.md"
    with open(readme_path, "w") as f:
        f.write("""# SQLite Vendored Sources

This directory contains vendored SQLite tool sources needed to build
syntaqlite-codegen. These files are copied from the main SQLite source
tree to make the crate self-contained for publishing to crates.io.

## Files

- `lemon.c` - Lemon parser generator (from SQLite's tool/lemon.c)
- `lempar.c` - Lemon parser template (from SQLite's tool/lempar.c)
- `mkkeywordhash.c` - Keyword hash generator (from SQLite's tool/mkkeywordhash.c)

## Updating

To update these files after upgrading SQLite:

```bash
python3 python/tools/vendor_sqlite_tools.py
```

## License

These files are part of SQLite and are in the public domain.
See https://www.sqlite.org/copyright.html
""")

    if verbose:
        print(f"Wrote: {readme_path.relative_to(ROOT_DIR)}")

    print(f"Successfully vendored {len(SQLITE_TOOLS)} SQLite tool files")
    return True


def main():
    parser = argparse.ArgumentParser(
        description="Vendor SQLite tool sources into syntaqlite-codegen crate"
    )
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Show detailed output"
    )
    args = parser.parse_args()

    success = vendor_files(verbose=args.verbose)
    return 0 if success else 1


if __name__ == "__main__":
    sys.exit(main())

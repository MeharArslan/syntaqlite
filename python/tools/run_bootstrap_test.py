#!/usr/bin/env python3
# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Bootstrap test: verify the codegen can regenerate all generated files from scratch.

Steps:
  1. Read tools/generated-files.txt to determine which files are generated.
  2. Back up all generated files.
  3. Delete them.
  4. Build syntaqlite-buildtools (must succeed without any generated files).
  5. Run tools/run-codegen (stages 1b + 2).
  6. Compare regenerated files against the backed-up originals.
  7. Restore originals on failure.

Usage:
    python3 python/tools/run_bootstrap_test.py
    tools/run-bootstrap-test
"""

import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


MANIFEST = "tools/generated-files.txt"


def read_manifest(project_root: Path) -> list[Path]:
    """Read the list of generated files from the manifest."""
    manifest_path = project_root / MANIFEST
    if not manifest_path.exists():
        print(f"Error: manifest not found at {manifest_path}", file=sys.stderr)
        sys.exit(1)

    paths = []
    for line in manifest_path.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        paths.append(project_root / line)
    return paths


def main() -> int:
    project_root = Path(__file__).parent.parent.parent

    generated = read_manifest(project_root)
    missing = [p for p in generated if not p.exists()]
    if missing:
        print("Warning: the following generated files are already absent:", file=sys.stderr)
        for p in missing:
            print(f"  {p.relative_to(project_root)}", file=sys.stderr)

    present = [p for p in generated if p.exists()]

    # Back up all present generated files into a temp directory.
    with tempfile.TemporaryDirectory(prefix="syntaqlite-bootstrap-") as backup_dir_str:
        backup_dir = Path(backup_dir_str)
        print(f"Backing up {len(present)} generated file(s) to {backup_dir}...")
        for src in present:
            rel = src.relative_to(project_root)
            dst = backup_dir / rel
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dst)

        # Delete all generated files.
        print(f"Deleting {len(present)} generated file(s)...")
        for p in present:
            p.unlink()

        # Verify syntaqlite-buildtools builds without any generated files.
        print("Building syntaqlite-buildtools (must succeed without generated files)...")
        result = subprocess.run(
            ["cargo", "build", "--release", "-p", "syntaqlite-buildtools"],
            cwd=project_root,
        )
        if result.returncode != 0:
            print(
                "FAIL: syntaqlite-buildtools failed to build without generated files.",
                file=sys.stderr,
            )
            _restore(backup_dir, project_root, present)
            return 1

        # Run the full codegen pipeline (stages 1b + 2).
        print("Running tools/run-codegen...")
        result = subprocess.run(
            [sys.executable, str(project_root / "python/tools/run_codegen.py")],
            cwd=project_root,
        )
        if result.returncode != 0:
            print("FAIL: run-codegen failed.", file=sys.stderr)
            _restore(backup_dir, project_root, present)
            return 1

        # Compare regenerated files against originals.
        print("Comparing regenerated files against originals...")
        diffs = []
        for src in present:
            rel = src.relative_to(project_root)
            backup = backup_dir / rel
            if not src.exists():
                diffs.append(f"  MISSING after regen: {rel}")
                continue
            if src.read_bytes() != backup.read_bytes():
                diffs.append(f"  DIFFERS: {rel}")

        # Check for any newly-absent files that were present before.
        regenerated = {p for p in generated if p.exists()}
        for p in present:
            if p not in regenerated:
                rel = p.relative_to(project_root)
                if f"  MISSING after regen: {rel}" not in diffs:
                    diffs.append(f"  MISSING after regen: {rel}")

        if diffs:
            print("FAIL: regenerated files differ from originals:", file=sys.stderr)
            for d in diffs:
                print(d, file=sys.stderr)
            _restore(backup_dir, project_root, present)
            return 1

        print(f"PASS: all {len(present)} generated file(s) regenerated identically.")
        return 0


def _restore(backup_dir: Path, project_root: Path, files: list[Path]) -> None:
    print("Restoring original files...", file=sys.stderr)
    for dst in files:
        rel = dst.relative_to(project_root)
        src = backup_dir / rel
        if src.exists():
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dst)


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""WorktreeCreate hook: creates a git worktree and hardlinks expensive directories.

Reads JSON from stdin: {"name": "...", "cwd": "/path/to/main/repo"}
Prints the absolute worktree path to stdout.

Only the following directories are hardlinked from the main repo:
  - third_party/  (~4GB, build dependencies: SQLite sources, clang-format, etc.)
  - target/       (~10GB, Cargo build artifacts)

Hardlinks work well for target/ because Cargo writes new artifacts via
temp-file-then-rename (atomic), so existing hardlinks are never modified in
place. Unchanged deps (90%+ of target/) stay shared; each worktree diverges
independently for recompiled crates.
"""

import json
import os
import shutil
import subprocess
import sys

# Directories to hardlink from the main repo into the worktree.
HARDLINK_DIRS = ["third_party", "target"]


def hardlink_tree(src, dst):
    """Recursively hardlink all files from src into dst.

    Creates the directory structure in dst and hardlinks each file.
    Falls back to copy if hardlinking fails (e.g. cross-device).
    """
    for dirpath, dirnames, filenames in os.walk(src):
        rel = os.path.relpath(dirpath, src)
        target = os.path.join(dst, rel) if rel != "." else dst
        os.makedirs(target, exist_ok=True)
        for fname in filenames:
            src_file = os.path.join(dirpath, fname)
            dst_file = os.path.join(target, fname)
            try:
                os.link(src_file, dst_file)
            except OSError:
                shutil.copy2(src_file, dst_file)


def main():
    request = json.load(sys.stdin)
    name = request["name"]
    cwd = request["cwd"]

    worktree_path = os.path.join(cwd, ".claude", "worktrees", name)
    branch_name = f"claude/{name}"

    # Create the worktree.
    result = subprocess.run(
        ["git", "worktree", "add", worktree_path, "-b", branch_name, "HEAD"],
        cwd=cwd,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(result.stderr, file=sys.stderr)
        sys.exit(1)

    # Hardlink expensive directories.
    for dirname in HARDLINK_DIRS:
        src = os.path.join(cwd, dirname)
        if os.path.isdir(src):
            dst = os.path.join(worktree_path, dirname)
            hardlink_tree(src, dst)

    # Print the worktree path for Claude Code.
    print(worktree_path)


if __name__ == "__main__":
    main()

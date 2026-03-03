#!/usr/bin/env python3
"""WorktreeRemove hook: removes a git worktree.

Reads JSON from stdin: {"worktree_path": "...", "cwd": "..."}
Attempts `git worktree remove`, falls back to rm -rf.
"""

import json
import shutil
import subprocess
import sys


def main():
    request = json.load(sys.stdin)
    worktree_path = request["worktree_path"]
    cwd = request["cwd"]

    result = subprocess.run(
        ["git", "worktree", "remove", "--force", worktree_path],
        cwd=cwd,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        # Fallback: remove manually and prune.
        shutil.rmtree(worktree_path, ignore_errors=True)
        subprocess.run(
            ["git", "worktree", "prune"],
            cwd=cwd,
            capture_output=True,
        )


if __name__ == "__main__":
    main()

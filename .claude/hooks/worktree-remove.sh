#!/bin/sh
# WorktreeRemove hook: remove a git worktree.
#
# Receives JSON on stdin with "worktree_path" field.

set -e

WORKTREE_PATH=$(cat | python3 -c "import json,sys; print(json.load(sys.stdin)['worktree_path'])")

git worktree remove --force "$WORKTREE_PATH" 2>/dev/null || {
  rm -rf "$WORKTREE_PATH"
  git worktree prune
}

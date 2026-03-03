#!/bin/sh
# WorktreeCreate hook: create a git worktree and symlink third_party/.
#
# Receives JSON on stdin with "name" and "cwd" fields.
# Must print the absolute worktree path to stdout.

set -e

NAME=$(cat | python3 -c "import json,sys; print(json.load(sys.stdin)['name'])")
CWD=$(pwd)

WORKTREE_PATH="$CWD/.claude/worktrees/$NAME"
BRANCH_NAME="claude/$NAME"

# Create the worktree.
git worktree add "$WORKTREE_PATH" -b "$BRANCH_NAME" HEAD >&2

# Track origin/main so `git push` goes directly to main.
git -C "$WORKTREE_PATH" branch --set-upstream-to=origin/main "$BRANCH_NAME" >&2

# Symlink third_party/ so build deps are available without copying.
if [ -d "$CWD/third_party" ] && [ ! -e "$WORKTREE_PATH/third_party" ]; then
  ln -s "$CWD/third_party" "$WORKTREE_PATH/third_party" >&2
fi

# Set up sparse-checkout to exclude shared instruction files.
git -C "$WORKTREE_PATH" sparse-checkout init --no-cone >&2
git -C "$WORKTREE_PATH" sparse-checkout set '/*' '!/.claude/rules/' '!/CLAUDE.md' '!/AGENTS.md' >&2

# Print path for Claude Code.
echo "$WORKTREE_PATH"

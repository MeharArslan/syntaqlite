---
name: cpfast
description: "Quickly commit and push current changes, skipping pre-push checks. Use when you want a fast save without running the full presubmit. WARNING: if origin/main starts failing, use /cp instead."
user_invocable: true
---

# commit-and-push-fast

Commit all current changes and push to the remote, skipping pre-push checks and
stash/sync steps for speed.

> **Note**: This skips `tools/pre-push` and the stash/pull/pop sync. If CI on
> `origin/main` starts failing after using this, run `/cp` instead — it runs the
> full presubmit gate and syncs with main before pushing.

## Instructions

1. **Check for changes**:
   ```sh
   git status
   git diff --stat
   git log --oneline -5
   ```

2. **Stage all changes**:
   ```sh
   git add -A
   ```

3. **Write a commit message** following the project convention:
   - Prefix with `synq: ` (lowercase)
   - Concise summary line describing the "why"
   - Add detail in the body for non-trivial changes

4. **Commit using a HEREDOC**:
   ```sh
   git commit -m "$(cat <<'EOF'
   synq: <summary>

   <optional body>
   EOF
   )"
   ```

5. **Push to main**:
   ```sh
   git push origin HEAD:main
   ```

6. **Report the commit hash** to the user.

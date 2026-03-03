---
name: cp
description: Commit all current changes and push to the remote. Use when the user asks to commit, push, save progress, or ship changes.
user_invocable: true
---

# commit-and-push

Commit all current changes and push to the remote. All pre-push checks must
pass before committing.

## Instructions

1. **Sync with origin/main**:
   ```sh
   git stash
   git pull origin main
   git stash pop
   ```
   If `git stash` complains that a file "needs merge" (unresolved conflict markers in the
   working tree), reset first then re-stash:
   ```sh
   git reset HEAD
   git stash
   ```
   If `stash pop` produces merge conflicts, resolve them before continuing.
   If the stash was empty (`No local changes to save`), skip the `stash pop`.

2. **Run pre-push checks with auto-fix** (quiet mode suppresses output on success):
   ```sh
   tools/pre-push --fix -q
   ```
   If this fails, fix the issues and re-run until it passes.
   Do NOT skip this step — it is the project's only gate against broken code.

3. **Check for changes**:
   ```sh
   git status
   git diff --stat
   git log --oneline -5
   ```

4. **Stage all changes** (including any fixes from step 2):
   ```sh
   git add -A
   ```

5. **Write a commit message** following the project convention:
   - Prefix with `synq: ` (lowercase)
   - Concise summary line describing the "why"
   - Add detail in the body for non-trivial changes

6. **Commit using a HEREDOC**:
   ```sh
   git commit -m "$(cat <<'EOF'
   synq: <summary>

   <optional body>
   EOF
   )"
   ```

7. **Push to remote**:
   ```sh
   git push
   ```

8. **Report the commit hash and branch** to the user.

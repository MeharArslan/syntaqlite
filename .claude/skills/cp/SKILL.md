---
name: cp
description: Commit all current changes and push to the remote. Use when the user asks to commit, push, save progress, or ship changes.
user_invocable: true
---

# commit-and-push

Commit all current changes and push to the remote. All pre-push checks must
pass before committing.

## Instructions

1. **Run pre-push checks with auto-fix** (quiet mode suppresses output on success):
   ```sh
   tools/pre-push --fix -q
   ```
   If this fails, fix the issues and re-run until it passes.
   Do NOT skip this step — it is the project's only gate against broken code.

2. **Check for changes**:
   ```sh
   git status
   git diff --stat
   git log --oneline -5
   ```

3. **Stage all changes** (including any fixes from step 1):
   ```sh
   git add -A
   ```

4. **Write a commit message** following the project convention:
   - Prefix with `synq: ` (lowercase)
   - Concise summary line describing the "why"
   - Add detail in the body for non-trivial changes

5. **Commit using a HEREDOC**:
   ```sh
   git commit -m "$(cat <<'EOF'
   synq: <summary>

   <optional body>
   EOF
   )"
   ```

6. **Push to remote**:
   ```sh
   git push
   ```

7. **Report the commit hash and branch** to the user.

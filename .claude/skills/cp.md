---
description: Commit all current changes and push to the remote
user_invocable: true
---

# commit-and-push

Commit all current changes and push to the remote.

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

5. **Push to remote**:
   ```sh
   git push
   ```

6. **Report the commit hash and branch** to the user.
